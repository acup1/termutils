#include <termutils.h>

void mywin_updater(window *win) {
  if (win->clicked) {
    wposwchar(win, win->clicked_y, win->clicked_x, L'+');
  }
}

int main() {
  init();
  cursset(0);
  enable_mouse();
  refresh();

  window *mywin = new_window(0);
  mywin->width = COLS;
  mywin->height = ROWS;
  mywin->clickable = 1;
  mywin->updater = mywin_updater;

  char c;
  while ((c = getch(10)) != 'q') {
    clear();
    render_windows();
    refresh();
  }
  restore();
  return 0;
}
