// chaude page: renders the transcript rows Rust sends (entry_push, entry_update, entry_cut) and posts the page's
// commands back. Plain script, no build step. The event and command shapes are core's (gui_bridge.rs).
(function () {
  "use strict";

  const chat = document.getElementById("chat");
  const column = document.getElementById("column");
  const input = document.getElementById("input");
  const sendBtn = document.getElementById("send");
  const stopBtn = document.getElementById("stop");
  const allowBtn = document.getElementById("allow");
  const denyBtn = document.getElementById("deny");
  const permission = document.getElementById("permission");
  const permissionText = document.getElementById("permission-text");
  const toast = document.getElementById("toast");

  // One DOM node per transcript row, by index (the same index as Rust's transcript).
  const rows = [];

  const send = (command) => window.ipc.postMessage(JSON.stringify(command));

  const el = (tag, cls, text) => {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined && text !== null) node.textContent = text;
    return node;
  };

  const nearBottom = () => chat.scrollHeight - chat.scrollTop - chat.clientHeight < 120;

  // Markdown as sanitized HTML. Images are dropped (no remote loads); links lose their href so a click cannot
  // navigate the webview away (the URL stays in the title).
  function renderMarkdown(target, text) {
    target.innerHTML = DOMPurify.sanitize(marked.parse(text, { gfm: true, breaks: true }), {
      FORBID_TAGS: ["img", "style", "form", "input", "iframe", "object", "embed"],
    });
    target.querySelectorAll("a[href]").forEach((a) => {
      a.title = a.getAttribute("href");
      a.removeAttribute("href");
    });
    target.querySelectorAll("pre").forEach(decorateCode);
  }

  // A code block: language label, highlighted code, and a copy button that asks Rust for the OS clipboard.
  function decorateCode(pre) {
    const code = pre.querySelector("code");
    if (!code) return;
    const lang = /language-(\S+)/.exec(code.className);
    if (window.hljs) hljs.highlightElement(code);
    const wrap = el("div", "codeblock");
    const bar = el("div", "codebar");
    bar.appendChild(el("span", "lang", lang ? lang[1] : "text"));
    const copy = el("button", "copy", "Copy");
    copy.type = "button";
    copy.addEventListener("click", () => send({ type: "copy", text: code.textContent }));
    bar.appendChild(copy);
    pre.replaceWith(wrap);
    wrap.append(bar, pre);
  }

  // Builds the DOM for one transcript row (EntryView, serde tag "kind").
  function makeRow(e) {
    switch (e.kind) {
      case "user": {
        const row = el("div", "row user");
        row.appendChild(el("div", "bubble", e.text));
        return row;
      }
      case "text": {
        const row = el("div", "row assistant" + (e.done ? "" : " streaming"));
        row.dataset.kind = "text";
        if (e.agent) row.appendChild(el("div", "who", "Agent"));
        const md = el("div", "md");
        row.appendChild(md);
        renderMarkdown(md, e.text);
        return row;
      }
      case "tool": {
        const row = el("details", "row tool");
        const state = e.end ? (e.ok === false ? "fail" : "done") : "running";
        row.classList.add(state);
        const summary = el("summary");
        summary.appendChild(el("span", "name", e.name));
        if (e.summary) summary.append(" ", el("span", "arg", e.summary));
        const verdict = e.end ? (e.ok === false ? " failed" : "") : " running";
        const tail = (e.end ? e.end.secs + "s" : "") + verdict;
        if (tail.trim()) summary.append(" ", el("span", "verdict", tail.trim()));
        row.appendChild(summary);
        return row;
      }
      case "tool_result": {
        const row = el("details", "row result");
        row.appendChild(el("summary", "", "Result"));
        row.appendChild(el("pre", "", e.text));
        return row;
      }
      case "agent": {
        const row = el("div", "row agent-card");
        row.appendChild(el("div", "", e.description + (e.done ? " (done)" : " (running)")));
        if (e.summary) row.appendChild(el("div", "", e.summary));
        return row;
      }
      case "notice":
        return el("div", "row notice", e.text);
      case "status":
        return el("div", "row status", e.text);
      case "error":
        return el("div", "row error", e.text);
      case "blank":
        return el("div", "row blank");
      default:
        return el("div", "row notice", "(unknown row)");
    }
  }

  function insertRow(index, node) {
    const stick = nearBottom();
    if (index >= rows.length) {
      column.appendChild(node);
      rows.push(node);
    } else {
      column.insertBefore(node, rows[index]);
      rows.splice(index, 0, node);
    }
    if (stick) chat.scrollTop = chat.scrollHeight;
  }

  // A streaming text row keeps its node and only re-renders its markdown; other rows are replaced in place (a
  // tool row keeps its open or closed state).
  function updateRow(index, e) {
    const old = rows[index];
    if (!old) {
      insertRow(index, makeRow(e));
      return;
    }
    const stick = nearBottom();
    if (e.kind === "text" && old.dataset.kind === "text") {
      old.classList.toggle("streaming", !e.done);
      if (e.agent && !old.querySelector(".who")) old.prepend(el("div", "who", "Agent"));
      renderMarkdown(old.querySelector(".md"), e.text);
    } else {
      const node = makeRow(e);
      if (old.tagName === "DETAILS" && node.tagName === "DETAILS") node.open = old.open;
      old.replaceWith(node);
      rows[index] = node;
    }
    if (stick) chat.scrollTop = chat.scrollHeight;
  }

  function cutRows(len) {
    while (rows.length > len) rows.pop().remove();
  }

  function showState(state) {
    stopBtn.hidden = !state.busy;
    const card = state.permission;
    permission.hidden = !card;
    if (card) {
      permissionText.textContent = card.tool + (card.summary ? ": " + card.summary : "");
    }
  }

  function submit() {
    const text = input.value.trim();
    if (!text) return;
    send({ type: "submit", text });
    input.value = "";
    resize();
  }

  function resize() {
    input.style.height = "auto";
    input.style.height = Math.min(input.scrollHeight, 200) + "px";
  }

  // Enter sends; Shift+Enter is a newline. While an IME composes text, Enter confirms the composition and must not
  // send (isComposing; keyCode 229 covers Safari's keydown right after composition ends).
  input.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" || e.shiftKey || e.isComposing || e.keyCode === 229) return;
    e.preventDefault();
    submit();
  });
  input.addEventListener("input", resize);
  sendBtn.addEventListener("click", submit);
  stopBtn.addEventListener("click", () => send({ type: "interrupt" }));
  allowBtn.addEventListener("click", () => send({ type: "permission_answer", allow: true, interrupt: false }));
  denyBtn.addEventListener("click", () => send({ type: "permission_answer", allow: false, interrupt: false }));

  // Rust calls onEvent for each event. An unknown type is ignored rather than thrown.
  const handlers = {
    pong() {},
    entry_push(ev) {
      insertRow(ev.index, makeRow(ev.entry));
    },
    entry_update(ev) {
      updateRow(ev.index, ev.entry);
    },
    entry_cut(ev) {
      cutRows(ev.len);
    },
    state(ev) {
      showState(ev.state);
    },
    picker_open() {},
    picker_close() {},
    note(ev) {
      toast.textContent = ev.text;
      toast.hidden = !ev.text;
    },
  };

  window.chaude = {
    send,
    onEvent(ev) {
      const handle = handlers[ev.type];
      if (handle) handle(ev);
    },
  };

  // The first ping makes Rust send the whole transcript and state, so the page starts in step with the core.
  send({ type: "ping" });
  input.focus();
})();
