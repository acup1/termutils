import termutils as tu


class snake_segment(tu.Window):
    def __init__(self):
        super().__init__(0)
        self.x=10
        self.y=10
        self.width=5
        self.height=3


class snake:
    tail=[]
    def __init__(self):
        pass

        

tu.init()
tu.cursset(0)

a=snake_segment()

tu.posprint(0,0,'')
tu.render_windows()
tu.refresh()

while tu.getch(100,0)!=ord('q'):tu.refresh()

tu.restore()
