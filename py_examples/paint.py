import termutils as tu


def radius_paint(y, x, radius, sym):
    for i in range(y - radius, y + radius + 1):
        for j in range(x - radius, x + radius + 1):
            if (i - y) ** 2 + (j - x) ** 2 <= radius**2:
                tu.posprint(i, j, sym)
    tu.setpos(y, x)


class pallete(tu.Window):
    colors = [
        [
            tu.BG_BLUE,
            tu.BG_BLACK,
            tu.BG_CYAN,
            tu.BG_GREEN,
            tu.BG_PURPLE,
            tu.BG_RED,
            tu.BG_WHITE,
            tu.BG_YELLOW,
        ],
        [
            tu.FG_BLUE,
            tu.FG_BLACK,
            tu.FG_CYAN,
            tu.FG_GREEN,
            tu.FG_PURPLE,
            tu.FG_RED,
            tu.FG_WHITE,
            tu.FG_YELLOW,
        ],
    ]

    def __init__(self) -> None:
        super().__init__(0)
        self.name = "palette"
        self.width = len(self.colors[0]) * 2 + 2
        self.height = 3
        self.clickable = 1
        for i, j in enumerate(self.colors[0]):
            self.pos_wchar(0, i, f"{i}")
            self.pos_wchar(0, i + len(self.colors[0]), f"{i}")

    def get_color(self, paint_bg, paint_fg):
        color = [paint_bg, paint_fg].copy()
        if self.get_clicked():
            x = self.get_clicked_x()
            if x < len(self.colors[0]) * 2:
                color[x // len(self.colors[0])] = self.colors[x // len(self.colors[0])][
                    x % len(self.colors[0])
                ]
        return color


tu.init()
tu.enable_mouse()
tu.cursset(0)


pallete_ins = pallete()

tu.poswchar(0, 0, "")
tu.refresh()

c = ""
sym = "*"
radius = 0
paint_bg = ""
paint_fg = ""

try:
    while c != "q":
        c = chr(tu.getch(100, 0))
        a = tu.get_mouse()
        if a.event == tu.LEFT or a.event == tu.SPAM_LEFT:
            radius_paint(a.y, a.x, radius, f"{paint_bg}{paint_fg}{sym}")
        elif a.event == tu.RIGHT or a.event == tu.SPAM_RIGHT:
            radius_paint(a.y, a.x, radius, f"{tu.RESET_COLOR} ")
            tu.setpos(a.y, a.x)

        if c == "c":
            tu.posprint(a.y, a.x, tu.RESET_COLOR)
            tu.clear()
        elif c == "r":
            print(tu.RESET_COLOR)
            paint_bg = ""
            paint_fg = ""
        elif c == "[":
            radius = max(0, radius - 1)
        elif c == "]":
            radius += 1
        elif c == "s":
            sym = chr(tu.getch(-1, 0))

        print(tu.RESET_COLOR)
        tu.render_windows()
        tu.poswchar(0, 0, "")
        tu.refresh()
        paint_bg, paint_fg = pallete_ins.get_color(paint_bg, paint_fg)

except Exception as e:
    with open("err.txt", "w") as f:
        f.write(f"{e}\n{pallete_ins.get_clicked_x()}")

tu.restore()
