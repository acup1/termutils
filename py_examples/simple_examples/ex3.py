import termutils as tu

tu.init()

tu.enable_mouse()
tu.cursset(0)
a = tu.Window(0)
a.x, a.y, a.width, a.height = 5, 5, 10, 5
a.dragable = 1
a.visible = 1
a.name = "qwe"
a.pos_wchar(0, 0, "1")
a.posprint(1, 0, "abcdefgh")
a.posprint(2, 0, "abcdefgh"[::-1])


# tu.poswchar(0, 0, "")
# tu.clear()
# tu.render_windows()

while (c := chr(tu.getch(10, 1))) != "q":
    if c == "c":
        a.clear()
    elif c == "e":
        del a
    tu.clear()
    tu.render_windows()

tu.restore()
