#!/usr/bin/env python3
"""Terminal pong (curses, stdlib only).

Controls: P1 = w/s, P2 = Up/Down arrows, q = quit at any time.
Score is shown on row 0. Runs at about 30 fps.
"""
import curses
import random
import time

FPS = 30
FRAME_S = 1.0 / FPS
MIN_W, MIN_H = 30, 10
PADDLE_H = 5
BALL_VX = 1.25      # columns per frame (was 1.0)
BALL_VY_MAX = 0.75  # rows per frame (was 0.6)


class Game:
    def __init__(self):
        self.s1 = 0
        self.s2 = 0
        self.size = None
        self.p1 = 1  # paddle top row
        self.p2 = 1
        self.bx = 0.0
        self.by = 0.0
        self.vx = 0.0
        self.vy = 0.0

    def clamp(self, h):
        top_max = max(1, h - 1 - PADDLE_H)
        self.p1 = max(1, min(self.p1, top_max))
        self.p2 = max(1, min(self.p2, top_max))

    def layout(self, h, w):
        # Called every frame; on resize, re-centre paddles and serve again.
        if self.size != (h, w):
            self.size = (h, w)
            self.p1 = self.p2 = 1 + (h - 1 - PADDLE_H) // 2
            self.clamp(h)
            self.serve(w, h, random.choice((-1, 1)))

    def serve(self, w, h, direction):
        self.bx = w / 2.0
        self.by = h / 2.0
        self.vx = BALL_VX * direction
        self.vy = random.uniform(-BALL_VY_MAX, BALL_VY_MAX)

    def key(self, k, h):
        if k in (ord('w'), ord('W')):
            self.p1 -= 1
        elif k in (ord('s'), ord('S')):
            self.p1 += 1
        elif k == curses.KEY_UP:
            self.p2 -= 1
        elif k == curses.KEY_DOWN:
            self.p2 += 1
        self.clamp(h)

    def step(self, h, w):
        top, bot = 1, h - 1
        c1, c2 = 2, w - 3  # paddle columns
        prev_x = self.bx
        self.bx += self.vx
        self.by += self.vy

        # Top/bottom wall bounce.
        if self.by < top:
            self.by = 2 * top - self.by
            self.vy = -self.vy
        elif self.by > bot:
            self.by = 2 * bot - self.by
            self.vy = -self.vy

        row = round(self.by)
        # Left paddle: ball crossing column c1 moving left.
        if self.vx < 0 and prev_x > c1 >= self.bx:
            if self.p1 <= row < self.p1 + PADDLE_H:
                self.bx = 2 * c1 - self.bx
                self.vx = -self.vx
                self._spin(row, self.p1)
        # Right paddle: ball crossing column c2 moving right.
        elif self.vx > 0 and prev_x < c2 <= self.bx:
            if self.p2 <= row < self.p2 + PADDLE_H:
                self.bx = 2 * c2 - self.bx
                self.vx = -self.vx
                self._spin(row, self.p2)

        # Scoring: ball left the field.
        if self.bx < 0:
            self.s2 += 1
            self.serve(w, h, -1)
        elif self.bx > w - 1:
            self.s1 += 1
            self.serve(w, h, 1)

    def _spin(self, row, ptop):
        # Spin: hitting near the paddle edge sends the ball off at a steeper angle.
        half = (PADDLE_H - 1) / 2.0
        offset = (row - (ptop + half)) / half
        self.vy = max(-BALL_VY_MAX, min(BALL_VY_MAX, offset * BALL_VY_MAX))

    def draw(self, scr, h, w):
        scr.erase()
        draw_score(scr, w, self.s1, self.s2)
        for r in range(1, h):
            if r % 2:
                put(scr, r, w // 2, '|')
        for r in range(self.p1, self.p1 + PADDLE_H):
            put(scr, r, 2, '#')
        for r in range(self.p2, self.p2 + PADDLE_H):
            put(scr, r, w - 3, '#')
        bx, by = round(self.bx), round(self.by)
        if 0 <= bx < w and 1 <= by < h:
            put(scr, by, bx, 'O')


def put(scr, y, x, s, attr=0):
    try:
        scr.addstr(y, x, s, attr)
    except curses.error:
        pass  # writing the bottom-right cell raises; ignore


def draw_score(scr, w, s1, s2):
    # Bold, centred score on row 0; also used in the small-window view.
    text = f"P1 {s1} : {s2} P2"[:w - 1]
    x = max(0, (w - len(text)) // 2)
    put(scr, 0, x, text, curses.A_BOLD)


def main(scr):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    scr.nodelay(True)
    scr.keypad(True)
    g = Game()
    while True:
        t0 = time.monotonic()
        h, w = scr.getmaxyx()
        g.layout(h, w)
        k = scr.getch()
        while k != -1:
            if k in (ord('q'), ord('Q')):
                return
            g.key(k, h)
            k = scr.getch()
        if h >= MIN_H and w >= MIN_W:
            g.step(h, w)
            g.draw(scr, h, w)
        else:
            scr.erase()
            draw_score(scr, w, g.s1, g.s2)
            if h > 1:
                put(scr, 1, 0, f"Window too small: need {MIN_W}x{MIN_H}, now {w}x{h}. q quits."[:w - 1])
        scr.refresh()
        time.sleep(max(0.0, FRAME_S - (time.monotonic() - t0)))


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        pass
