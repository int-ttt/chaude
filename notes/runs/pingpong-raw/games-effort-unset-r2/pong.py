#!/usr/bin/env python3
"""Terminal 2-player pong (curses, stdlib only).

Controls: P1 w/s, P2 Up/Down, q quits from any state.
r restarts after a game is over.
"""
import curses
import math
import random
import time

BALL_SPEED = 4.0      # cells per frame
WIN_SCORE = 7
PADDLE_H = 5
PADDLE_STEP = 2
FPS = 30
SERVE_DELAY = 1.0     # seconds before serve after a point
MIN_W, MIN_H = 30, 10
TOP = 1               # row 0 is the score line


def fold(v, lo, hi):
    """Reflect v into [lo, hi] (bounce off both edges)."""
    span = hi - lo
    if span <= 0:
        return lo
    v = (v - lo) % (2 * span)
    if v > span:
        v = 2 * span - v
    return lo + v


def clamp(v, lo, hi):
    return max(lo, min(hi, v))


class Game:
    def __init__(self, scr):
        self.scr = scr
        self.h = self.w = 0
        self.bot = TOP
        self.p1x = self.p2x = 0
        self.p = [None, None]          # paddle top rows (ints)
        self.score = [0, 0]
        self.ball_x = self.ball_y = 0.0
        self.vx = self.vy = 0.0
        self.state = "serve"           # serve | play | over
        self.serve_at = time.monotonic() + SERVE_DELAY
        self.serve_dir = random.choice((-1, 1))
        self.winner = None

    # geometry ---------------------------------------------------------
    def layout(self):
        h, w = self.scr.getmaxyx()
        self.h, self.w = h, w
        self.bot = h - 2               # row h-1 is the status line
        self.p1x, self.p2x = 2, w - 3
        lo, hi = TOP, self.bot - PADDLE_H + 1
        for i in (0, 1):
            if self.p[i] is None:
                self.p[i] = (lo + hi) // 2
            self.p[i] = clamp(self.p[i], lo, hi)
        if self.state == "serve":
            self.ball_x = w / 2
            self.ball_y = (TOP + self.bot) / 2
        else:
            self.ball_x = clamp(self.ball_x, self.p1x, self.p2x)
            self.ball_y = clamp(self.ball_y, TOP, self.bot)

    def paddle_range(self):
        return TOP, self.bot - PADDLE_H + 1

    # input ------------------------------------------------------------
    def handle_key(self, ch):
        lo, hi = self.paddle_range()
        if ch in (ord("w"), ord("W")):
            self.p[0] = clamp(self.p[0] - PADDLE_STEP, lo, hi)
        elif ch in (ord("s"), ord("S")):
            self.p[0] = clamp(self.p[0] + PADDLE_STEP, lo, hi)
        elif ch == curses.KEY_UP:
            self.p[1] = clamp(self.p[1] - PADDLE_STEP, lo, hi)
        elif ch == curses.KEY_DOWN:
            self.p[1] = clamp(self.p[1] + PADDLE_STEP, lo, hi)
        elif ch in (ord("r"), ord("R")) and self.state == "over":
            self.score = [0, 0]
            self.winner = None
            self.state = "serve"
            self.serve_at = time.monotonic() + SERVE_DELAY

    # game logic -------------------------------------------------------
    def launch(self):
        self.state = "play"
        self.vy = random.uniform(-0.4, 0.4) * BALL_SPEED
        self.vx = self.serve_dir * math.sqrt(max(BALL_SPEED ** 2 - self.vy ** 2, 0.01))

    def paddle_hit(self, idx, y):
        top = self.p[idx]
        return top - 0.5 <= y < top + PADDLE_H - 0.5

    def bounce_off(self, idx, y, direction):
        center = self.p[idx] + (PADDLE_H - 1) / 2
        offset = clamp((y - center) / ((PADDLE_H - 1) / 2), -1.0, 1.0)
        self.vy = offset * BALL_SPEED * 0.6
        self.vx = direction * math.sqrt(max(BALL_SPEED ** 2 - self.vy ** 2, 0.01))

    def point(self, idx):
        self.score[idx] += 1
        self.serve_dir = 1 if idx == 0 else -1   # serve toward the player who conceded
        if self.score[idx] >= WIN_SCORE:
            self.state = "over"
            self.winner = idx
        else:
            self.state = "serve"
            self.serve_at = time.monotonic() + SERVE_DELAY

    def step(self, now):
        if self.state == "serve":
            if now >= self.serve_at:
                self.launch()
            return
        if self.state != "play":
            return

        lo, hi = TOP, self.bot
        x0, y0 = self.ball_x, self.ball_y
        vy0 = self.vy
        x1 = x0 + self.vx
        raw_y = y0 + vy0
        if raw_y < lo or raw_y > hi:
            self.vy = -vy0
        y1 = fold(raw_y, lo, hi)

        # Segment crossing test so fast balls cannot tunnel past a paddle.
        if self.vx < 0 and x1 <= self.p1x:
            t = (self.p1x - x0) / (x1 - x0)
            y_hit = fold(y0 + vy0 * t, lo, hi)
            if self.paddle_hit(0, y_hit):
                self.ball_x, self.ball_y = self.p1x + 1, y_hit
                self.bounce_off(0, y_hit, 1)
            else:
                self.point(1)
            return
        if self.vx > 0 and x1 >= self.p2x:
            t = (self.p2x - x0) / (x1 - x0)
            y_hit = fold(y0 + vy0 * t, lo, hi)
            if self.paddle_hit(1, y_hit):
                self.ball_x, self.ball_y = self.p2x - 1, y_hit
                self.bounce_off(1, y_hit, -1)
            else:
                self.point(0)
            return

        self.ball_x, self.ball_y = x1, y1

    # drawing ----------------------------------------------------------
    def put(self, y, x, text, attr=0):
        try:
            self.scr.addstr(y, x, text, attr)
        except curses.error:
            pass  # writing the bottom-right cell can raise; ignore

    def draw(self):
        scr = self.scr
        scr.erase()
        h, w = self.h, self.w
        # score is drawn first, on row 0 (TOP=1 keeps play field off it), in every state
        score = f"P1 {self.score[0]} : {self.score[1]} P2"
        self.put(0, max((w - len(score)) // 2, 0), score, curses.A_BOLD)
        if h < MIN_H or w < MIN_W:
            msg = f"Terminal too small ({w}x{h}); need {MIN_W}x{MIN_H}. q to quit"
            self.put(max(h // 2, 0), 0, msg[: max(w - 1, 0)])
            return

        for y in range(TOP, self.bot + 1):
            self.put(y, w // 2, "|")
        for i, x in enumerate((self.p1x, self.p2x)):
            for r in range(self.p[i], self.p[i] + PADDLE_H):
                self.put(r, x, "#")
        if self.state in ("serve", "play"):
            self.put(int(round(self.ball_y)), int(round(self.ball_x)), "O")

        if self.state == "over":
            status = f"P{self.winner + 1} wins! r: restart  q: quit"
        else:
            status = "P1 w/s   P2 Up/Down   q quit"
        self.put(h - 1, 0, status[: max(w - 1, 0)])

    # main loop --------------------------------------------------------
    def run(self):
        scr = self.scr
        scr.nodelay(True)
        scr.keypad(True)
        try:
            curses.curs_set(0)
        except curses.error:
            pass
        frame = 1.0 / FPS
        while True:
            t0 = time.monotonic()
            self.layout()
            ch = scr.getch()
            while ch != -1:
                if ch in (ord("q"), ord("Q")):
                    return
                self.handle_key(ch)
                ch = scr.getch()
            if self.h >= MIN_H and self.w >= MIN_W:
                self.step(t0)
            self.draw()
            scr.refresh()
            time.sleep(max(0.0, frame - (time.monotonic() - t0)))


def main(scr):
    Game(scr).run()


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        pass
