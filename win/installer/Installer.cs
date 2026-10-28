// Claude Lean installer for Windows: a small WinForms window that runs install.ps1 / fresh_start.ps1
// from the package embedded as the resource "pkg.zip". C# 5 only, so the compiler that ships with
// Windows (.NET Framework 4, csc.exe) can build it; no Python or runtime is bundled.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Windows.Forms;

static class Program
{
    [STAThread]
    static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        string pkg;
        try { pkg = Extract(); }
        catch (Exception e) { MessageBox.Show("패키지를 풀지 못했습니다: " + e.Message, "Claude Lean"); return; }
        Application.Run(new InstallerForm(pkg));
    }

    // %LOCALAPPDATA%\ClaudeLean\pkg, replaced on every run so it always matches this exe
    static string Extract()
    {
        string root = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ClaudeLean");
        string dir = Path.Combine(root, "pkg");
        if (Directory.Exists(dir)) Directory.Delete(dir, true);
        Directory.CreateDirectory(root);
        using (Stream s = Assembly.GetExecutingAssembly().GetManifestResourceStream("pkg.zip"))
        using (ZipArchive z = new ZipArchive(s))
            z.ExtractToDirectory(dir);
        return dir;
    }
}

class Choice
{
    public string Key, Title;
    public string[] Values, Labels;
    public Choice(string key, string title, string[] values, string[] labels) { Key = key; Title = title; Values = values; Labels = labels; }
}

class InstallerForm : Form
{
    static readonly Choice[] Choices = {
        new Choice("main", "메인 모델", new[] { "\"opus\"", "\"sonnet\"" }, new[] { "Opus (판단이 많은 작업)", "Sonnet (지시가 명확한 작업, 더 저렴)" }),
        new Choice("style", "답변 방식", new[] { "\"explain\"", "\"concise\"" }, new[] { "설명형 (일반 Opus처럼 풀어서)", "간결형 (5줄 이내)" }),
        new Choice("inbox", "정보 문장 메모", new[] { "true", "false" }, new[] { "켜기", "끄기" }),
        new Choice("reset_alert", "맥락 리셋 안내 (/exit 후 다시 시작)", new[] { "100000", "120000", "0" }, new[] { "100k에서", "120k에서", "끄기" }),
        new Choice("history_export", "대화 기록 txt 자동 저장", new[] { "true", "false" }, new[] { "켜기", "끄기" }),
        new Choice("nesting", "Sonnet·Opus → Haiku 연계", new[] { "true", "false" }, new[] { "켜기", "끄기" }),
        new Choice("start_mode", "설치 직후 모드", new[] { "\"lean\"", "\"normal\"" }, new[] { "절약 모드", "일반 모드 (절약 규칙 꺼짐)" }),
    };

    readonly string pkg;
    readonly ListBox folders = new ListBox();
    readonly RadioButton actInstall = new RadioButton(), actFresh = new RadioButton();
    readonly Dictionary<string, RadioButton[]> radios = new Dictionary<string, RadioButton[]>();
    readonly TextBox log = new TextBox();
    readonly Button go = new Button();

    public InstallerForm(string pkg)
    {
        this.pkg = pkg;
        string ver = File.ReadAllText(Path.Combine(pkg, "VERSION")).Trim();
        Text = "Claude Lean " + ver + " 설치";
        Font = new Font("Malgun Gothic", 9F);
        ClientSize = new Size(760, 720);
        StartPosition = FormStartPosition.CenterScreen;
        AutoScaleMode = AutoScaleMode.Dpi;

        var page = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = 1, Padding = new Padding(10) };
        Controls.Add(page);

        page.Controls.Add(new Label { Text = "프로젝트 폴더", AutoSize = true });
        folders.Height = 70; folders.Dock = DockStyle.Fill;
        page.Controls.Add(folders);
        var fbar = new FlowLayoutPanel { AutoSize = true };
        var add = new Button { Text = "폴더 추가…", AutoSize = true };
        var del = new Button { Text = "선택 삭제", AutoSize = true };
        add.Click += delegate { AddFolder(); };
        del.Click += delegate { if (folders.SelectedIndex >= 0) folders.Items.RemoveAt(folders.SelectedIndex); };
        fbar.Controls.Add(add); fbar.Controls.Add(del);
        page.Controls.Add(fbar);

        var act = new GroupBox { Text = "작업", Dock = DockStyle.Fill, Height = 64 };
        var actFlow = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.TopDown };
        actInstall.Text = "설치·업그레이드만"; actInstall.AutoSize = true; actInstall.Checked = true;
        actFresh.Text = "기존 대화 정리 후 새로 시작 (Claude 창을 닫고 실행)"; actFresh.AutoSize = true;
        actFlow.Controls.Add(actInstall); actFlow.Controls.Add(actFresh);
        act.Controls.Add(actFlow);
        page.Controls.Add(act);

        var grid = new TableLayoutPanel { ColumnCount = 2, Dock = DockStyle.Fill, AutoSize = true };
        grid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 50));
        grid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 50));
        foreach (Choice c in Choices)
        {
            var box = new GroupBox { Text = c.Title, Dock = DockStyle.Fill, Height = 40 + 22 * c.Labels.Length };
            var flow = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.TopDown };
            var rs = new RadioButton[c.Labels.Length];
            for (int i = 0; i < rs.Length; i++)
            {
                rs[i] = new RadioButton { Text = c.Labels[i], AutoSize = true, Checked = i == 0 };
                flow.Controls.Add(rs[i]);
            }
            box.Controls.Add(flow);
            grid.Controls.Add(box);
            radios[c.Key] = rs;
        }
        page.Controls.Add(grid);

        go.Text = "설치"; go.Width = 120; go.Height = 32;
        go.Click += delegate { Run(); };
        page.Controls.Add(go);
        log.Multiline = true; log.ReadOnly = true; log.ScrollBars = ScrollBars.Vertical; log.Dock = DockStyle.Fill; log.Height = 150;
        page.Controls.Add(log);
    }

    void AddFolder()
    {
        using (var d = new FolderBrowserDialog { Description = "설치할 프로젝트 폴더" })
        {
            if (d.ShowDialog(this) != DialogResult.OK || folders.Items.Contains(d.SelectedPath)) return;
            folders.Items.Add(d.SelectedPath);
            if (folders.Items.Count == 1) LoadSaved(d.SelectedPath);
        }
    }

    // previous choices of an already installed project become the defaults
    void LoadSaved(string folder)
    {
        string p = Path.Combine(folder, ".claude", "claude-lean.options.json");
        if (!File.Exists(p)) return;
        string json = File.ReadAllText(p, Encoding.UTF8);
        foreach (Choice c in Choices)
        {
            Match m = Regex.Match(json, "\"" + c.Key + "\"\\s*:\\s*(\"[^\"]*\"|true|false|\\d+)");
            if (!m.Success) continue;
            int i = Array.IndexOf(c.Values, m.Groups[1].Value);
            if (i >= 0) radios[c.Key][i].Checked = true;
        }
    }

    string Selected(Choice c)
    {
        RadioButton[] rs = radios[c.Key];
        for (int i = 0; i < rs.Length; i++) if (rs[i].Checked) return c.Values[i];
        return c.Values[0];
    }

    void Run()
    {
        if (folders.Items.Count == 0) { log.Text = "프로젝트 폴더를 먼저 추가하세요."; return; }
        var parts = new List<string>();
        foreach (Choice c in Choices) parts.Add("\"" + c.Key + "\": " + Selected(c));
        string opt = Path.Combine(Path.GetTempPath(), "claude-lean-options.json");
        File.WriteAllText(opt, "{" + string.Join(", ", parts) + "}", new UTF8Encoding(false));
        string main = Selected(Choices[0]).Trim('"');
        var dirs = new List<string>();
        foreach (object f in folders.Items) dirs.Add((string)f);
        bool fresh = actFresh.Checked;

        go.Enabled = false;
        log.Text = "설치 중…" + Environment.NewLine;
        new Thread(delegate ()
        {
            var sb = new StringBuilder();
            if (fresh)
                sb.Append(Ps("fresh_start.ps1", Quote(dirs) + " -Main " + main, opt));
            else
                foreach (string d in dirs) sb.Append(Ps("install.ps1", "\"" + d + "\" -Main " + main, opt)).Append(Environment.NewLine);
            File.Delete(opt);
            string text = sb.ToString().Replace("\r\n", "\n").Replace("\n", Environment.NewLine);
            BeginInvoke((MethodInvoker)delegate
            {
                log.Text = text + Environment.NewLine + (text.Contains("✗") ? "일부 작업이 실패했습니다. 위 내용을 확인하세요." : "완료. 각 프로젝트 폴더에서 claude를 새로 시작하세요.");
                go.Enabled = true;
            });
        }) { IsBackground = true }.Start();
    }

    static string Quote(List<string> dirs)
    {
        var q = new List<string>();
        foreach (string d in dirs) q.Add("\"" + d + "\"");
        return string.Join(" ", q);
    }

    string Ps(string script, string args, string opt)
    {
        var psi = new ProcessStartInfo("powershell.exe",
            "-NoProfile -ExecutionPolicy Bypass -File \"" + Path.Combine(pkg, script) + "\" " + args)
        {
            UseShellExecute = false, CreateNoWindow = true, WorkingDirectory = pkg,
            RedirectStandardOutput = true, RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8, StandardErrorEncoding = Encoding.UTF8,
        };
        psi.EnvironmentVariables["CLAUDE_LEAN_OPTIONS"] = opt;
        psi.EnvironmentVariables["PYTHONUTF8"] = "1";
        using (Process p = Process.Start(psi))
        {
            string err = null;
            var t = new Thread(delegate () { err = p.StandardError.ReadToEnd(); });
            t.Start();
            string output = p.StandardOutput.ReadToEnd();
            p.WaitForExit(); t.Join();
            return output + (string.IsNullOrEmpty(err) ? "" : Environment.NewLine + err);
        }
    }
}
