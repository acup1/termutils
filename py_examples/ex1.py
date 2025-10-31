import termutils as tu
from random import randint

tu.init()

while tu.getch(100, 0) != ord("q"):
    tu.poswchar(randint(1, tu.ROWS()), randint(1, tu.COLS()), "󱄅")
    tu.refresh()

tu.restore()
