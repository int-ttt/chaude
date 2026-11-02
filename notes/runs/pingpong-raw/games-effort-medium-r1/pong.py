#!/usr/bin/env python3
"""Terminal ping-pong: curses, stdlib only.

Controls: P1 w/s, P2 Up/Down, q quits.
Run: python3 pong.py
"""
import curses
import random
import time

FPS = 30
BALL_VX = 1.625         # cells per frame, horizontal speed
BALL_VY_MAX = 0.975     # cells per frame, vertical speed cap
PADDLE_H = 5
PADDLE_STEP = 2         # cells moved per key press
MIN_W, MIN_H = 30, 10   # smaller than this shows a message instead of the game
TOP = 1                 # screen row 0 holds the score
SERVE_FRAMES = FPS      # pause after each point (about 1 s)


class Game:
    """Playfield logic in cell units. No curses dependency."""

    def __init__(self, width, field_h, rng=None):
        self.rng = rng or random.Random()
        self.score = [0, 0]
        self.paddle = [0, 0]   # top playfield row of each paddle
        self.bx = self.by = 0.0
        self.vx = self.vy = 0.0
        self.pause = 0
        self.resize(width, field_h)
        self.paddle = [(field_h - PADDLE_H) // 2] * 2
        self.serve(self.rng.choice((-1, 1)))

    def resize(self, width, field_h):
        self.w = width
        self.fh = field_h
        self.lx = 1
        self.rx = width - 2
        self.move_paddle(0, 0)
        self.move_paddle(1, 0)
        if self.bx > width - 1 or self.by > field_h - 1:
            self.serve(self.rng.choice((-1, 1)))

    def serve(self, direction):
        """Centre the ball, serve toward `direction` (-1 left, 1 right)."""
        self.bx = self.w / 2
        self.by = self.fh / 2
        self.vx = BALL_VX * direction
        self.vy = self.rng.uniform(-0.5, 0.5) * BALL_VY_MAX
        self.pause = SERVE_FRAMES

    def move_paddle(self, side, delta):
        top_max = max(0, self.fh - PADDLE_H)
        self.paddle[side] = min(max(self.paddle[side] + delta, 0), top_max)

    def step(self):
        if self.pause > 0:
            self.pause -= 1
            return
        px, py = self.bx, self.by
        self.bx += self.vx
        self.by += self.vy
        # Paddle columns: the ball may jump over a column in one frame,
        # so test the segment it crossed, not just its end point.
        if self.vx < 0 and px > self.lx >= self.bx:
            if self._cross(0, px, py):
                return
        elif self.vx > 0 and px < self.rx <= self.bx:
            if self._cross(1, px, py):
                return
        # Top and bottom walls reflect.
        if self.by < 0:
            self.by = -self.by
            self.vy = -self.vy
        elif self.by > self.fh - 1:
            self.by = 2 * (self.fh - 1) - self.by
            self.vy = -self.vy

    def _cross(self, side, px, py):
        """Ball crossed paddle `side`'s column. Return True if a point was scored."""
        col = self.lx if side == 0 else self.rx
        t = (px - col) / (px - self.bx)
        yc = py + (self.by - py) * t          # row where the ball crossed the column
        top = self.paddle[side]
        if not (top - 0.5 <= yc <= top + PADDLE_H - 0.5):
            self.score[1 - side] += 1
            self.serve(-1 if side == 0 else 1)   # serve toward the player who conceded
            return True
        self.bx = 2 * col - self.bx          # reflect off the paddle
        self.vx = -self.vx
        half = (PADDLE_H - 1) / 2
        offset = (yc - (top + half)) / half  # -1 top edge .. +1 bottom edge
        self.vy = max(-BALL_VY_MAX, min(BALL_VY_MAX, offset * BALL_VY_MAX))
        return False


def put(scr, y, x, text, attr=0):
    try:
        scr.addstr(y, x, text, attr)
    except curses.error:
        pass  # writing the bottom-right cell raises; harmless


def draw(scr, game, h, w):
    scr.erase()
    text = f"P1 {game.score[0]} : {game.score[1]} P2"
    put(scr, 0, max(0, (w - len(text)) // 2), text[:w], curses.A_BOLD)
    for side, col in ((0, game.lx), (1, game.rx)):
        for r in range(PADDLE_H):
            put(scr, TOP + game.paddle[side] + r, col, "#")
    put(scr, TOP + round(game.by), round(game.bx), "O")


def main(scr):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    scr.nodelay(True)
    scr.keypad(True)
    frame = 1.0 / FPS
    game = None
    while True:
        t0 = time.monotonic()
        h, w = scr.getmaxyx()
        quit_now = False
        while True:  # drain every key pressed since the last frame
            k = scr.getch()
            if k == -1:
                break
            if k in (ord("q"), ord("Q")):
                quit_now = True
            elif game is not None:
                if k in (ord("w"), ord("W")):
                    game.move_paddle(0, -PADDLE_STEP)
                elif k in (ord("s"), ord("S")):
                    game.move_paddle(0, PADDLE_STEP)
                elif k == curses.KEY_UP:
                    game.move_paddle(1, -PADDLE_STEP)
                elif k == curses.KEY_DOWN:
                    game.move_paddle(1, PADDLE_STEP)
        if quit_now:
            break

        if h >= MIN_H and w >= MIN_W:
            if game is None:
                game = Game(w, h - TOP)
            else:
                game.resize(w, h - TOP)
            game.step()
            draw(scr, game, h, w)
        else:
            scr.erase()
            put(scr, 0, 0, f"Terminal too small (need {MIN_W}x{MIN_H}). q quits.")

        scr.refresh()
        time.sleep(max(0.0, frame - (time.monotonic() - t0)))


def run():
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    run()
