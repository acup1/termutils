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

# tu.poswchar(0, 0, "")
# tu.clear()
# tu.render_windows()

while tu.getch(10, 1) != ord("q"):
    tu.clear()
    tu.render_windows()

tu.restore()
