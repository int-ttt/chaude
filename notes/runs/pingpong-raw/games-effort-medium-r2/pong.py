#!/usr/bin/env python3
"""Terminal pong (curses, stdlib only).

Controls: P1 w/s, P2 up/down arrows, q quits.
Run: python3 pong.py
"""
import curses
import random

MIN_W = 40
MIN_H = 12
FPS = 30
PADDLE_H = 4
PADDLE_STEP = 1
LEFT_X = 2
BALL_VX = 2.1125  # was 1.625 (+30%)
BALL_VY = 0.65  # was 0.5 (+30%); spin max scales with it


def reset_ball(w, h, direction):
    bx = w / 2.0
    by = h / 2.0
    vy = random.choice((-BALL_VY, BALL_VY))
    return bx, by, BALL_VX * direction, vy


def spin(by, paddle_top):
    # -1 at paddle top edge, +1 at bottom edge; scaled into a vertical speed
    offset = (by - paddle_top) / (PADDLE_H - 1) * 2 - 1
    return offset * BALL_VY * 1.5


def put(scr, y, x, text, attr=0):
    try:
        scr.addstr(y, x, text, attr)
    except curses.error:
        pass  # writing the bottom-right cell raises after drawing; ignore


def draw(scr, h, w, s1, s2, p1, p2, bx, by):
    scr.erase()
    title = "P1 %d : %d P2" % (s1, s2)
    put(scr, 0, max(0, (w - len(title)) // 2), title, curses.A_BOLD)
    for y in range(1, h):
        put(scr, y, w // 2, ":")
    right_x = w - 1 - LEFT_X
    for i in range(PADDLE_H):
        put(scr, p1 + i, LEFT_X, "|")
        put(scr, p2 + i, right_x, "|")
    put(scr, int(round(by)), int(round(bx)), "o")
    scr.refresh()


def main(scr):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    scr.nodelay(True)
    scr.keypad(True)

    h, w = scr.getmaxyx()
    s1 = s2 = 0
    p1 = p2 = max(1, (h - PADDLE_H) // 2)
    bx, by, vx, vy = reset_ball(w, h, random.choice((-1, 1)))

    while True:
        key = scr.getch()
        while key != -1:
            if key in (ord("q"), ord("Q")):
                return
            if key in (ord("w"), ord("W")):
                p1 -= PADDLE_STEP
            elif key in (ord("s"), ord("S")):
                p1 += PADDLE_STEP
            elif key == curses.KEY_UP:
                p2 -= PADDLE_STEP
            elif key == curses.KEY_DOWN:
                p2 += PADDLE_STEP
            key = scr.getch()

        h, w = scr.getmaxyx()
        if h < MIN_H or w < MIN_W:
            scr.erase()
            msg = "Terminal too small: need %dx%d, have %dx%d. Resize or press q." % (
                MIN_W, MIN_H, w, h)
            put(scr, 0, 0, msg[:max(0, w - 1)])
            scr.refresh()
            curses.napms(1000 // FPS)
            continue

        top, bottom = 1, h - 1
        p1 = min(max(p1, top), h - PADDLE_H)
        p2 = min(max(p2, top), h - PADDLE_H)
        bx = min(max(bx, 0.0), w - 1.0)
        by = min(max(by, top * 1.0), bottom * 1.0)

        prev_bx = bx
        bx += vx
        by += vy
        if by < top:
            by = 2 * top - by
            vy = abs(vy)
        elif by > bottom:
            by = 2 * bottom - by
            vy = -abs(vy)

        right_x = w - 1 - LEFT_X
        if vx < 0 and bx <= LEFT_X and prev_bx > LEFT_X - 1:
            if p1 <= by < p1 + PADDLE_H:
                bx = 2 * LEFT_X - bx
                vx = abs(vx)
                vy = spin(by, p1)
        elif vx > 0 and bx >= right_x and prev_bx < right_x + 1:
            if p2 <= by < p2 + PADDLE_H:
                bx = 2 * right_x - bx
                vx = -abs(vx)
                vy = spin(by, p2)

        if bx < 0:
            s2 += 1
            bx, by, vx, vy = reset_ball(w, h, random.choice((-1, 1)))
        elif bx > w - 1:
            s1 += 1
            bx, by, vx, vy = reset_ball(w, h, random.choice((-1, 1)))

        draw(scr, h, w, s1, s2, p1, p2, bx, by)
        curses.napms(1000 // FPS)


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        pass
