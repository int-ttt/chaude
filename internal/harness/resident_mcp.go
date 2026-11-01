package harness

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
	"path/filepath"
)

// residentMCPConfig returns the --mcp-config file that makes the main child run `chaude mcp-resident <sock>`.
// The first call starts the unix-socket listener that serves mcpReply; later calls reuse it.
func (r *Repl) residentMCPConfig() (string, error) {
	r.mcpMu.Lock()
	defer r.mcpMu.Unlock()
	if r.mcpCfg != "" {
		return r.mcpCfg, nil
	}
	exe, err := os.Executable()
	if err != nil {
		return "", err
	}
	dir, err := os.MkdirTemp("", "chaude-mcp-")
	if err != nil {
		return "", err
	}
	sock := filepath.Join(dir, "mcp.sock")
	ln, err := net.Listen("unix", sock)
	if err != nil {
		return "", err
	}
	cfg := map[string]any{"mcpServers": map[string]any{"chaude": map[string]any{
		"command": exe, "args": []string{"mcp-resident", sock}}}}
	b, err := json.Marshal(cfg)
	if err != nil {
		ln.Close()
		return "", err
	}
	path := filepath.Join(dir, "mcp.json")
	if err := os.WriteFile(path, b, 0o600); err != nil {
		ln.Close()
		return "", err
	}
	go r.serveMCP(ln)
	r.mcpCfg = path
	return path, nil
}

// serveMCP accepts bridge connections until the listener closes.
func (r *Repl) serveMCP(ln net.Listener) {
	for {
		conn, err := ln.Accept()
		if err != nil {
			return
		}
		go r.serveMCPConn(conn)
	}
}

// serveMCPConn answers newline-delimited JSON-RPC lines from one bridge until it closes.
func (r *Repl) serveMCPConn(conn net.Conn) {
	defer conn.Close()
	sc := bufio.NewScanner(conn)
	sc.Buffer(make([]byte, 0, 64<<10), 4<<20)
	for sc.Scan() {
		if resp := r.mcpReply(sc.Bytes()); resp != nil {
			if _, err := conn.Write(append(resp, '\n')); err != nil {
				return
			}
		}
	}
}

// MCPResidentBridge is `chaude mcp-resident <socket>`: it copies MCP lines between the main child's
// stdin/stdout and the repl's socket (see residentMCPConfig).
func MCPResidentBridge(args []string) int {
	if len(args) != 1 {
		fmt.Fprintln(os.Stderr, "usage: chaude mcp-resident <socket>")
		return 2
	}
	conn, err := net.Dial("unix", args[0])
	if err != nil {
		fmt.Fprintln(os.Stderr, "mcp-resident:", err)
		return 1
	}
	defer conn.Close()
	done := make(chan struct{})
	go func() {
		_, _ = io.Copy(os.Stdout, conn)
		close(done)
	}()
	_, _ = io.Copy(conn, os.Stdin)
	if uc, ok := conn.(*net.UnixConn); ok {
		_ = uc.CloseWrite() // EOF to the repl side: it closes after the last response
	}
	<-done
	return 0
}
