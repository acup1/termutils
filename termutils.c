#include "termutils.h"
#include <locale.h>
#include <math.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <time.h>
#include <unistd.h>
#include <wchar.h>

static struct termios original, changed;
// static int winmode = 0;
static int wincount = 0;
window **windows;

int ROWS, COLS;
struct mouse MOUSE;

void init() {
  setlocale(LC_ALL, "");
  tcgetattr(0, &original);
  changed = original;
  changed.c_lflag &= ~(ICANON | ECHO);
  changed.c_cc[VMIN] = 1;
  changed.c_cc[VTIME] = 0;
  tcsetattr(STDIN_FILENO, TCSANOW, &changed);
  atexit(restore);

  signal(SIGINT, SIG_IGN);

  // struct sigaction sa;
  // sa.sa_handler = size_handler;
  // sigemptyset(&sa.sa_mask);
  // sa.sa_flags = 0;
  // sigaction(SIGWINCH, &sa, NULL);

  getsize(&ROWS, &COLS);
  printf("\033[?1049h");
  clear();
  refresh();
}

void restore() {
  printf(RESET_COLOR);
  disable_mouse();
  printf("\033[?1049l");
  cursset(1);
  tcsetattr(STDIN_FILENO, TCSANOW, &original);
  for (int i = 0; i < wincount; i++) {
    wclear(windows[i]);
    if (windows[i]->name)
      free(windows[i]->name);
    free(windows[i]);
  }
  refresh();
  exit(0);
}

void getsize(int *height, int *width) {
  // printf("\033[18t");
  // fflush(stdout);
  // if (scanf("\033[8;%d;%dt", height, width) != 2) {
  //   *width = 20;
  //   *height = 10;
  // }
  struct winsize w;
  if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &w) == -1) {
    *width = 20;
    *height = 10;
  } else {
    *width = w.ws_col;
    *height = w.ws_row;
  }
}

void cursset(int visibility) {
  if (visibility)
    printf("\x1b[?25h");
  else
    printf("\x1b[?25l");
  refresh();
}

void setpos(int y, int x) { printf("\033[%d;%dH", y + 1, x + 1); }

void posprint(int y, int x, char *s) {
  setpos(y, x);
  printf("%s", s);
}

void poschar(int y, int x, char c) {
  setpos(y, x);
  printf("%c", c);
}

void poswchar(int y, int x, wchar_t c) {
  setpos(y, x);
  printf("%lc", c);
}

void clear() { printf("\033[2J"); }

void refresh() {
  fflush(stdout);
  getsize(&ROWS, &COLS);
}

void box(int y, int x, int h, int w, int style) {
  for (int i = fmin(y, x); i < fmax(y + h - 1, x + w - 1); i++) {
    if (i < y + h - 1 && y < i) {
      posprint(i, x, "│");
      posprint(i, x + w - 1, "│");
    }
    if (i < x + w - 1 && x < i) {
      posprint(y, i, "─");
      posprint(y + h - 1, i, "─");
    }
  }
  posprint(y, x, style ? "┌" : "╭");
  posprint(y + h - 1, x, style ? "└" : "╰");
  posprint(y, x + w - 1, style ? "┐" : "╮");
  posprint(y + h - 1, x + w - 1, style ? "┘" : "╯");
}

int getch(int timeout) {
  clock_t start = clock();
  int ret = KEY_FAIL;
  struct pollfd fds[1];
  fds[0].fd = STDIN_FILENO;
  fds[0].events = POLLIN;
  int p = poll(fds, 1, timeout);
  unsigned char buf[15];
  int bytes;
  switch (p) {
  case 0:
    ret = p;
    break;
  default:
    // ret = getchar();
    // if (ret == 27) {
    //   char c1 = getchar();
    //   if (c1 == 91) {
    //     char c2 = getchar();
    //     switch (c2) {
    //     case 'A':
    //       ret = KEY_UP;
    //       break;
    //     case 'B':
    //       ret = KEY_DOWN;
    //       break;
    //     case 'C':
    //       ret = KEY_RIGHT;
    //       break;
    //     case 'D':
    //       ret = KEY_LEFT;
    //       break;
    //     default:
    //       ret = KEY_FAIL;
    //       break;
    //     }
    //   } else {
    //     printf("%d", c1);
    //   }
    // }
    bytes = read(STDIN_FILENO, buf, sizeof(buf) - 1);
    if (bytes > 0) {
      if (bytes >= 6 && buf[0] == 27 && buf[1] == '[' && buf[2] == 'M') {
        // mouse
        ret = MOUSE_EVENT;
        unsigned char button = buf[3];
        MOUSE.x = buf[4] - 32;
        MOUSE.y = buf[5] - 32;
        MOUSE.x--;
        MOUSE.y--;
        switch (button) {
        case 32:
          MOUSE.event = LEFT;
          break;
        case 33:
          MOUSE.event = MIDDLE;
          break;
        case 34:
          MOUSE.event = RIGHT;
          break;
        case 35:
          MOUSE.event = IDLE;
          break;
        case 64:
          MOUSE.event = SPAM_LEFT;
          break;
        case 65:
          MOUSE.event = SPAM_MIDDLE;
          break;
        case 66:
          MOUSE.event = SPAM_RIGHT;
          break;
        case 96:
          MOUSE.event = WHEEL_UP;
          break;
        case 97:
          MOUSE.event = WHEEL_DOWN;
          break;
        }
      } else if (bytes == 1) {
        ret = buf[0];
      }
      break;
    }
  }
  while ((double)((clock() - start) * 1000) / CLOCKS_PER_SEC < timeout)
    ;
  return ret;
}

void enable_mouse() {
  MOUSE.x = 0;
  MOUSE.y = 0;
  MOUSE.event = IDLE;
  printf("\033[?1000h\033[?1002h");
  fflush(stdout);
}

void disable_mouse() {
  printf("\033[?1002l\033[?1000l");
  fflush(stdout);
}

window *new_window(int layer) {
  getsize(&ROWS, &COLS);
  window *win = malloc(sizeof(window));
  win->id = wincount;
  win->name = NULL;
  win->updater = NULL;
  win->border = 0;
  win->filling = 1;
  win->visible = 1;
  win->layer = layer;
  win->content_length = 0;
  win->content_offset_x = 0;
  win->content_offset_y = 0;
  win->dragable = 0;
  win->clickable = 0;
  win->clicked = 0;
  win->clicked_x = 0;
  win->clicked_y = 0;

  if (wincount > 0)
    windows = realloc(windows, sizeof(window *) * (++wincount));
  else
    windows = malloc(sizeof(window *) * (++wincount));
  *(windows + (wincount - 1)) = win;

  for (int i = 0; i < wincount - 1; i++)
    for (int j = 0; j < wincount - 1; j++)
      if (windows[j]->layer > windows[j + 1]->layer) {
        window *swap = windows[j];
        windows[j] = windows[j + 1];
        windows[j + 1] = swap;
      }

  return win;
}

void wposwchar(window *win, int y, int x, wchar_t c) {
  int draw = 1;
  for (int i = 0; i < win->content_length && draw; i++)
    if (win->content[i]->sym == c && win->content[i]->x == x &&
        win->content[i]->y == y)
      draw = 0;
    else if (win->content[i]->sym != c && win->content[i]->x == x &&
             win->content[i]->y == y) {
      draw = 0;
      win->content[i]->sym = c;
    }
  if (draw) {
    cell *current_cell = malloc(sizeof(cell));
    current_cell->y = y;
    current_cell->x = x;
    current_cell->sym = c;
    if (win->content_length > 0)
      win->content =
          realloc(win->content, sizeof(cell *) * (++win->content_length));
    else
      win->content = malloc(sizeof(cell *) * (++win->content_length));
    win->content[win->content_length - 1] = current_cell;
  }
}

void wclear(window *win) {
  if (win)
    if (win->content_length) {
      for (int i = 0; i < win->content_length; i++)
        free(win->content[i]);
      win->content_length = 0;
      free(win->content);
    }
}

void render_windows() {
  for (int i = 0; i < wincount; i++) {
    window *win = windows[i];

    if (win->visible) {
      if (win->clickable) {
        win->clicked = 0;
        if (MOUSE.event == LEFT &&
            (win->y <= MOUSE.y && MOUSE.y <= (win->y + win->height - 1)) &&
            (win->x <= MOUSE.x && MOUSE.x <= (win->x + win->width - 1))) {
          win->clicked = 1;
          win->clicked_y = MOUSE.y - win->y - 1;
          win->clicked_x = MOUSE.x - win->x - 1;
        }
      }

      if (win->dragable) {
        if (MOUSE.event == LEFT && MOUSE.y == win->y &&
            (win->x <= MOUSE.x && MOUSE.x <= win->x + win->width)) {
          win->drag = 1;
          win->drag_ofset = MOUSE.x - win->x;
        }
        if (win->drag) {
          if (MOUSE.event == IDLE)
            win->drag = 0;
          win->x = MOUSE.x - win->drag_ofset;
          win->y = MOUSE.y;
        }
      }

      if (win->x < 0)
        win->x = 0;
      if (win->y < 0)
        win->y = 0;
      if (win->x + win->width > COLS)
        win->x = COLS - win->width;
      if (win->y + win->height > ROWS)
        win->y = ROWS - win->height;

      if (win->filling)
        for (int i = 0; i < win->height - 2; i++)
          for (int j = 0; j < win->width - 2; j++)
            poschar(win->y + i + 1, win->x + j + 1, ' ');

      for (int i = 0; i < win->content_length; i++)
        if ((win->content[i]->y < win->height - 2) &&
            (win->content[i]->x < win->width - 2))
          poswchar(win->y + win->content[i]->y + 1,
                   win->x + win->content[i]->x + 1, win->content[i]->sym);

      if (win->border != -1)
        box(win->y, win->x, win->height, win->width, win->border);
      if (win->name)
        posprint(win->y, win->x + 1, win->name);
    }
    if (win->updater)
      win->updater(win);
  }
}
