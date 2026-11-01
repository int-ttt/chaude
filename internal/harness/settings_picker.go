package harness

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// settingsPick is the /settings cursor picker: a working copy of the settings file. Esc saves it.
type settingsPick struct {
	s    chaudeSettings
	sel  int
	path string
}

var settingsRetainChoices = []int{0, 7, 30, 90, 365}
var settingsModelChoices = []string{"opus", "sonnet", "fable"}

// settingsRows are the picker rows, in display order.
const settingsRows = 3

// openSettings loads the settings file into the picker. Safe while busy: it only reads and edits a copy.
func (m *tuiModel) openSettings() {
	path := settingsFileFor(m.r.a.Cwd)
	m.stPick = &settingsPick{s: loadChaudeSettings(path), path: path}
}

// settingsKey handles one key for the open picker. Esc/q saves and closes.
func (m *tuiModel) settingsKey(k string) {
	p := m.stPick
	switch k {
	case "up", "k":
		p.sel = (p.sel + settingsRows - 1) % settingsRows
	case "down", "j":
		p.sel = (p.sel + 1) % settingsRows
	case "left", "h":
		p.change(-1)
	case "right", "l", "enter":
		p.change(1)
	case "esc", "ctrl+c", "q":
		m.closeSettings()
	}
}

// closeSettings saves the picker's copy and reports what applies when.
func (m *tuiModel) closeSettings() {
	p := m.stPick
	m.stPick = nil
	if p == nil {
		return
	}
	msg := "설정 저장: " + p.path + " · 보존 기간·추가 coder 수·메인 모델은 다음 세션 시작 시 적용"
	if p.path == "" {
		msg = "설정 경로를 알 수 없음 (작업 폴더 없음)"
	} else if err := saveChaudeSettings(p.path, p.s); err != nil {
		msg = "설정 저장 실패: " + err.Error()
	}
	m.blocks = append(m.blocks, block{"note", msg, false})
	m.refresh()
}

// change steps the value of the selected row by dir (+1/-1), wrapping.
func (p *settingsPick) change(dir int) {
	switch p.sel {
	case 0:
		i := 0
		for j, v := range settingsRetainChoices {
			if v == p.s.SessionRetentionDays {
				i = j
			}
		}
		n := len(settingsRetainChoices)
		p.s.SessionRetentionDays = settingsRetainChoices[(i+dir+n)%n]
	case 1:
		n := settingsMaxExtraCoders + 1
		p.s.ExtraCoders = (p.s.ExtraCoders + dir + n) % n
	case 2:
		i := 0
		for j, v := range settingsModelChoices {
			if v == p.s.MainModel {
				i = j
			}
		}
		n := len(settingsModelChoices)
		p.s.MainModel = settingsModelChoices[(i+dir+n)%n]
	}
}

// settingsLines renders the picker into h lines.
func (m *tuiModel) settingsLines(h int) string {
	p := m.stPick
	orangeSt := lipgloss.NewStyle().Foreground(orange)
	l := []string{orangeSt.Render("Settings  (↑/↓ wheel move · ←/→ or enter change · esc save & close)")}
	rows := []string{
		fmt.Sprintf("session_retention_days  %d일 (0 = 영구 보존)", p.s.SessionRetentionDays),
		fmt.Sprintf("extra_coders            %d (추가 coder (270초) 수, 0-8)", p.s.ExtraCoders),
		fmt.Sprintf("main_model              %s", p.s.MainModel),
	}
	for i, r := range rows {
		t := "  " + r
		if i == p.sel {
			t = orangeSt.Render("> " + r)
		} else {
			t = dimSt.Render(t)
		}
		l = append(l, ansiTrunc(t, m.w))
	}
	l = append(l, dimSt.Render("  변경은 다음 세션 시작 시 적용됩니다."))
	for len(l) < h {
		l = append(l, "")
	}
	if len(l) > h {
		l = l[:h]
	}
	return strings.Join(l, "\n")
}
