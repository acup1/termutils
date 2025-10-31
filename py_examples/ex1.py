import termutils as tu
from random import randint

tu.init()

while tu.getch(100, 0)!=ord('q'):
    tu.poswchar(randint(1,tu.get_rows()),randint(1,tu.get_cols()),'󱄅')
    tu.refresh()

tu.restore()
