import termutils as tu
from math import cos, sin, pi


class snake_segment(tu.Window):
    def __init__(self, size):
        super().__init__(0)
        self.width, self.height = size
        self.x, self.y = 0, 0
        self.always_on_screen = 0


class snake:
    tail = []
    direction = 0
    pre_direction = 0
    size = (5, 3)

    def __init__(self):
        pass

    def __add__(self, inc):
        for _ in range(inc):
            seg = snake_segment(self.size)
            if len(self.tail):
                seg.x = int(
                    self.tail[-1].x + cos(self.direction * pi / 2) * self.size[0]
                )
                seg.y = int(
                    self.tail[-1].y - sin(self.direction * pi / 2) * self.size[1]
                )
            self.tail.append(seg)
        return self

    def move(self):
        if len(self.tail) > 1:
            self.direction = self.pre_direction
            seg = self.tail.pop(0)
            seg.x = int(self.tail[-1].x + cos(self.direction * pi / 2) * self.size[0])
            seg.y = int(self.tail[-1].y - sin(self.direction * pi / 2) * self.size[1])
            self.tail.append(seg)

    def turn(self, direction):
        if abs(self.direction - direction) != 2:
            self.pre_direction = direction

    def __call__(self, action, *args):
        if action == "move":
            self.move()
        elif action == "turn":
            self.turn(args[0])


tu.init()
tu.cursset(0)

s = snake()
s += 3

frame = 0

tu.posprint(0, 0, "")
tu.render_windows()
c = chr(tu.getch(2, 1))

while c != "q":
    if c == "d":
        s("turn", 0)
    elif c == "w":
        s("turn", 1)
    elif c == "a":
        s("turn", 2)
    elif c == "s":
        s("turn", 3)

    if frame % 50 == 0:
        s("move")
        frame = 0
    tu.clear()
    tu.render_windows()

    frame += 1
    c = chr(tu.getch(2, 1))


tu.restore()
