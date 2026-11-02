#!/usr/bin/env python3
"""Terminal pong. curses + stdlib only.

Controls: P1 w/s, P2 Up/Down, r restart, q quit.
Run: python3 pong.py
"""
import curses
import math
import random
import time

BALL_SPEED = 5.0          # cells per frame
MAX_ANGLE = math.radians(60)
SERVE_ANGLE = math.radians(30)
PADDLE_H = 5
PADDLE_STEP = 2           # cells per key press
WIN_SCORE = 7
FPS = 30
SERVE_DELAY = 1.0         # seconds before each serve
MIN_W, MIN_H = 30, 10
TOP = 1                   # row 0 is the score line


class Pong:
    def __init__(self, stdscr):
        self.scr = stdscr
        self.p1 = self.p2 = 0.0
        self.layout()
        self.reset()

    def layout(self):
        self.h, self.w = self.scr.getmaxyx()
        self.bot = self.h - 1
        self.lx = 2
        self.rx = self.w - 3
        self.p1 = self.clamp_p(self.p1)
        self.p2 = self.clamp_p(self.p2)

    def pmax(self):
        return max(TOP, self.bot - PADDLE_H + 1)

    def clamp_p(self, y):
        return min(max(y, TOP), self.pmax())

    def reset(self):
        self.score = [0, 0]
        self.p1 = self.p2 = (TOP + self.pmax()) / 2
        self.start_point(time.monotonic())

    def center_ball(self):
        self.bx = self.w / 2
        self.by = (TOP + self.bot) / 2
        self.vx = self.vy = 0.0

    def start_point(self, now):
        self.center_ball()
        self.state = "serve"
        self.serve_at = now + SERVE_DELAY

    def launch(self):
        d = random.choice((-1, 1))
        a = random.uniform(-SERVE_ANGLE, SERVE_ANGLE)
        self.vx = d * BALL_SPEED * math.cos(a)
        self.vy = BALL_SPEED * math.sin(a)

    def point(self, idx, now):
        self.score[idx] += 1
        if self.score[idx] >= WIN_SCORE:
            self.state = "over"
            self.center_ball()
        else:
            self.start_point(now)

    def try_hit(self, px, ptop, dirn, ox, nx, oy, ny):
        # ball crosses paddle column at px; check the crossing row
        t = (px - ox) / (nx - ox)
        yc = oy + (ny - oy) * t
        if not (ptop - 0.5 <= yc <= ptop + PADDLE_H - 0.5):
            return False
        mid = (PADDLE_H - 1) / 2
        rel = max(-1.0, min(1.0, (yc - ptop - mid) / mid))
        a = rel * MAX_ANGLE
        self.bx, self.by = float(px), yc
        self.vx = dirn * BALL_SPEED * math.cos(a)
        self.vy = BALL_SPEED * math.sin(a)
        return True

    def step(self, now):
        if self.state == "serve":
            if now < self.serve_at:
                return
            self.state = "play"
            self.launch()
        if self.state != "play":
            return
        ox, oy = self.bx, self.by
        nx, ny = ox + self.vx, oy + self.vy
        if ny < TOP:
            ny = 2 * TOP - ny
            self.vy = -self.vy
        elif ny > self.bot:
            ny = 2 * self.bot - ny
            self.vy = -self.vy
        ny = min(max(ny, TOP), self.bot)
        if self.vx < 0 and ox > self.lx >= nx:
            if self.try_hit(self.lx, self.p1, 1, ox, nx, oy, ny):
                return
        elif self.vx > 0 and ox < self.rx <= nx:
            if self.try_hit(self.rx, self.p2, -1, ox, nx, oy, ny):
                return
        self.bx, self.by = nx, ny
        if nx < 0:
            self.point(1, now)
        elif nx > self.w - 1:
            self.point(0, now)

    def handle_key(self, key):
        """Return False when the game should quit."""
        if key in (ord("q"), ord("Q")):
            return False
        if key in (ord("r"), ord("R")):
            self.reset()
        elif key in (ord("w"), ord("W")):
            self.p1 = self.clamp_p(self.p1 - PADDLE_STEP)
        elif key in (ord("s"), ord("S")):
            self.p1 = self.clamp_p(self.p1 + PADDLE_STEP)
        elif key == curses.KEY_UP:
            self.p2 = self.clamp_p(self.p2 - PADDLE_STEP)
        elif key == curses.KEY_DOWN:
            self.p2 = self.clamp_p(self.p2 + PADDLE_STEP)
        return True

    def put(self, y, x, s, attr=0):
        if 0 <= y < self.h and 0 <= x < self.w:
            try:
                self.scr.addstr(y, x, s, attr)
            except curses.error:
                pass

    def draw(self):
        scr = self.scr
        scr.erase()
        text = "P1 %d : %d P2" % (self.score[0], self.score[1])
        self.put(0, max(0, (self.w - len(text)) // 2), text, curses.A_BOLD)
        if self.h < MIN_H or self.w < MIN_W:
            self.put(TOP, 0, "Terminal too small: need %dx%d" % (MIN_W, MIN_H))
            scr.refresh()
            return
        for y in range(TOP, self.bot + 1, 2):
            self.put(y, self.w // 2, ":")
        for i in range(PADDLE_H):
            self.put(int(self.p1) + i, self.lx, "#")
            self.put(int(self.p2) + i, self.rx, "#")
        bx = min(max(int(round(self.bx)), 0), self.w - 1)
        by = min(max(int(round(self.by)), TOP), self.bot)
        self.put(by, bx, "O")
        if self.state == "over":
            winner = "P1" if self.score[0] > self.score[1] else "P2"
            msg = "%s WINS  r: restart  q: quit" % winner
            self.put((self.h + TOP) // 2, (self.w - len(msg)) // 2, msg, curses.A_BOLD)
        scr.refresh()


def run(stdscr):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    stdscr.nodelay(True)
    stdscr.keypad(True)
    game = Pong(stdscr)
    frame = 1.0 / FPS
    while True:
        t0 = time.monotonic()
        key = stdscr.getch()
        while key != -1:
            if not game.handle_key(key):
                return
            key = stdscr.getch()
        game.layout()
        if game.h >= MIN_H and game.w >= MIN_W:
            game.step(t0)
        game.draw()
        time.sleep(max(0.0, frame - (time.monotonic() - t0)))


if __name__ == "__main__":
    curses.wrapper(run)
