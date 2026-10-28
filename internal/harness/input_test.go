package harness

import "testing"

func TestInputWidthMixed(t *testing.T) {
	b := &inputBox{width: 10}
	b.Insert([]rune("ab한글cd"))
	if r, c := b.Cursor(); r != 0 || c != 8 {
		t.Fatalf("cursor %d,%d want 0,8", r, c)
	}
	b.Left()
	b.Left()
	b.Left() // before 글
	if _, c := b.Cursor(); c != 4 {
		t.Fatalf("col %d want 4", c)
	}
	b.Backspace() // removes 한
	if b.Value() != "ab글cd" {
		t.Fatalf("got %q", b.Value())
	}
	if _, c := b.Cursor(); c != 2 {
		t.Fatalf("col %d want 2", c)
	}
}

func TestInputWrapWide(t *testing.T) {
	b := &inputBox{width: 6}
	b.Insert([]rune("가나다라마")) // 3 per row (6 cells)
	rs := b.rows()
	if len(rs) != 2 || rs[0].end != 3 {
		t.Fatalf("rows %v", rs)
	}
	if r, c := b.Cursor(); r != 1 || c != 4 {
		t.Fatalf("cursor %d,%d want 1,4", r, c)
	}
	b.Insert([]rune("x")) // 4+... row1 = 라마 + x = 5 cells fits
	if len(b.rows()) != 2 {
		t.Fatal("unexpected wrap")
	}
	if !b.MoveRow(-1) {
		t.Fatal("no up")
	}
	if r, c := b.Cursor(); r != 0 || c != 4 {
		t.Fatalf("after up %d,%d", r, c)
	}
}

func TestInputNewline(t *testing.T) {
	b := &inputBox{width: 20}
	b.Insert([]rune("안녕\r\nhi\n"))
	if b.Value() != "안녕\nhi\n" {
		t.Fatalf("got %q", b.Value())
	}
	if r, c := b.Cursor(); r != 2 || c != 0 {
		t.Fatalf("cursor %d,%d", r, c)
	}
	b.Backspace()
	b.Home()
	if _, c := b.Cursor(); c != 0 {
		t.Fatal("home")
	}
	b.End()
	if _, c := b.Cursor(); c != 2 {
		t.Fatal("end")
	}
}
