#!/usr/bin/env python3
"""Terminal ping-pong (curses, stdlib only).

Left paddle: W / S. Right paddle: Up / Down. q quits. r restarts after game over.
"""
import curses
import math
import random
import time

BALL_SPEED = 3.2     # cells per frame
PADDLE_H = 5
PADDLE_STEP = 2      # cells per key press
WIN_SCORE = 7        # first to this many points wins
FPS = 30
SERVE_DELAY = 1.0    # seconds before serve after a point
MIN_W, MIN_H = 30, 10


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


class Game:
    def __init__(self, h, w, now):
        self.new_match(h, w, now)

    def new_match(self, h, w, now):
        top, bot = 1, max(1, h - 2)
        self.left = self.right = 0
        self.over = False
        self.winner = ""
        self.lp = self.rp = (top + bot) / 2 - PADDLE_H / 2
        self.reset_ball(h, w, random.choice((-1, 1)), now)

    def reset_ball(self, h, w, d, now):
        top, bot = 1, max(1, h - 2)
        self.bx = (w - 1) / 2
        self.by = (top + bot) / 2
        a = random.uniform(-0.7, 0.7)
        self.vx = d * BALL_SPEED * math.cos(a)
        self.vy = BALL_SPEED * math.sin(a)
        self.serve_at = now + SERVE_DELAY

    def bounce(self, d, paddle_top):
        # angle depends on where the ball hits the paddle
        center = paddle_top + (PADDLE_H - 1) / 2
        off = clamp((self.by - center) / ((PADDLE_H - 1) / 2), -1, 1)
        a = off * 0.9
        self.vx = d * BALL_SPEED * math.cos(a)
        self.vy = BALL_SPEED * math.sin(a)

    def key(self, ch, h, w, now):
        if ch in (ord("w"), ord("W")):
            self.lp -= PADDLE_STEP
        elif ch in (ord("s"), ord("S")):
            self.lp += PADDLE_STEP
        elif ch == curses.KEY_UP:
            self.rp -= PADDLE_STEP
        elif ch == curses.KEY_DOWN:
            self.rp += PADDLE_STEP
        elif ch in (ord("r"), ord("R")) and self.over:
            self.new_match(h, w, now)

    def step(self, h, w, now):
        top, bot = 1, max(1, h - 2)
        # keep everything inside the current size (resize-safe)
        self.lp = clamp(self.lp, top, bot - PADDLE_H + 1)
        self.rp = clamp(self.rp, top, bot - PADDLE_H + 1)
        self.bx = clamp(self.bx, 0, w - 1)
        self.by = clamp(self.by, top, bot)
        if self.over or now < self.serve_at:
            return

        ox = self.bx
        self.bx += self.vx
        self.by += self.vy
        # top/bottom walls
        if self.by < top:
            self.by = 2 * top - self.by
            self.vy = -self.vy
        elif self.by > bot:
            self.by = 2 * bot - self.by
            self.vy = -self.vy
        self.by = clamp(self.by, top, bot)

        lx, rx = 2, w - 3
        # left paddle: ball crossed its column moving left
        if self.vx < 0 and self.bx <= lx <= ox:
            if self.lp - 0.5 <= self.by <= self.lp + PADDLE_H - 0.5:
                self.bounce(1, self.lp)
                self.bx = 2 * lx - self.bx
        # right paddle: ball crossed its column moving right
        elif self.vx > 0 and ox <= rx <= self.bx:
            if self.rp - 0.5 <= self.by <= self.rp + PADDLE_H - 0.5:
                self.bounce(-1, self.rp)
                self.bx = 2 * rx - self.bx

        if self.bx < 0 or self.bx > w - 1:
            right_scored = self.bx < 0
            if right_scored:
                self.right += 1
            else:
                self.left += 1
            if max(self.left, self.right) >= WIN_SCORE:
                self.over = True
                self.winner = "RIGHT" if right_scored else "LEFT"
            else:
                # serve toward the player who conceded
                self.reset_ball(h, w, -1 if right_scored else 1, now)


def put(scr, y, x, s, attr=0):
    try:
        scr.addstr(y, x, s, attr)
    except curses.error:
        pass


def draw(scr, g, h, w):
    scr.erase()
    if h < MIN_H or w < MIN_W:
        put(scr, 0, 0, "Terminal too small: need %dx%d (q quits)" % (MIN_W, MIN_H))
        scr.refresh()
        return
    top, bot = 1, h - 2
    score = "P1 %d : %d P2" % (g.left, g.right)
    put(scr, 0, max(0, (w - len(score)) // 2), score, curses.A_BOLD)
    for y in range(top, bot + 1, 2):
        put(scr, y, w // 2, ":")
    for i in range(PADDLE_H):
        put(scr, int(round(g.lp)) + i, 2, "#")
        put(scr, int(round(g.rp)) + i, w - 3, "#")
    put(scr, int(round(g.by)), int(round(g.bx)), "O", curses.A_BOLD)
    if g.over:
        hint = "%s WINS!  r: restart  q: quit" % g.winner
    else:
        hint = "W/S: left  Up/Down: right  q: quit"
    put(scr, h - 1, 0, hint[: w - 1], curses.A_DIM)
    scr.refresh()


def main(scr):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    scr.nodelay(True)
    scr.keypad(True)
    frame = 1.0 / FPS
    h, w = scr.getmaxyx()
    g = Game(h, w, time.monotonic())
    while True:
        t0 = time.monotonic()
        h, w = scr.getmaxyx()
        ch = scr.getch()
        while ch != -1:
            if ch in (ord("q"), ord("Q")):
                return
            g.key(ch, h, w, t0)
            ch = scr.getch()
        if h >= MIN_H and w >= MIN_W:
            g.step(h, w, t0)
        draw(scr, g, h, w)
        time.sleep(max(0.0, frame - (time.monotonic() - t0)))


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        pass
