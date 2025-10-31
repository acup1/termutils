import termutils as tu
from math import cos,pi

tu.init()
tu.cursset(0)

for i in range(tu.get_cols()):
    tu.poswchar(int(tu.get_rows()//2*cos(3*2*pi*(1/tu.get_cols())*i))+tu.get_rows()//2,i,'*')
    tu.refresh()

while tu.getch(100, 0)!=ord('q'):
    pass

tu.restore()
