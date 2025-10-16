#include <termutils.h>

void mywin_updater(window *win) {
  if (win->clicked) {
    wposwchar(win, win->clicked_y, win->clicked_x, L'+');
  }
}

void wfillslashes(window_p win) {
  wclear(win);
  for (int i = 0; i < win->height - 3; i++)
    for (int j = 0; j < win->width - 3; j++)
      if ((i + j) % 5 == 0)
        wposwchar(win, i, j, L'/');
}

int main() {
  init();
  cursset(0);
  enable_mouse();
  refresh();

  window_p mywin = new_window(0);
  mywin->y = 0;
  mywin->x = 0;
  mywin->width = 60;
  mywin->height = 30;
  // mywin->border = -1;
  mywin->always_on_sreen = 0;
  mywin->clickable = 1;
  mywin->dragable = 1;
  mywin->filling = 0;
  mywin->updater = mywin_updater;

  char c;
  while ((c = getch(15, 1)) != 'q') {
    if (c == 'c')
      wclear(mywin);
    else if (c == '/')
      wfillslashes(mywin);
    else if (c == 'f')
      wtogglefullscreen(mywin);

    clear();
    render_windows();
    poschar(0, 0, '#');
    poschar(0, COLS, '#');
    poschar(ROWS, COLS, '#');
    poschar(ROWS, 0, '#');
    refresh();
  }
  restore();
  return 0;
}
