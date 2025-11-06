import termutils as tu
from math import cos, pi

tu.init()
tu.cursset(0)

for i in range(tu.COLS()):
    tu.poswchar(
        int(tu.ROWS() // 2 * cos(2 * pi * (1 / tu.ROWS()) * i)) + tu.ROWS() // 2,
        i,
        "*",
    )
    tu.refresh()

while tu.getch(100, 0) != ord("q"):
    pass

tu.restore()
