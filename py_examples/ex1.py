import termutils as tu

tu.init()

while tu.getch(100, 0)!=ord('q'):
    pass

tu.restore()
