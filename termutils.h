#include <wchar.h>

#ifndef TERMUTILS_H
#define TERMUTILS_H

// COLORS
#define FG_BLACK "\033[30m"
#define FG_RED "\033[31m"
#define FG_GREEN "\033[32m"
#define FG_YELLOW "\033[33m"
#define FG_BLUE "\033[34m"
#define FG_PURPLE "\033[35m"
#define FG_CYAN "\033[36m"
#define FG_WHITE "\033[37m"

#define BG_BLACK "\033[40m"
#define BG_RED "\033[41m"
#define BG_GREEN "\033[42m"
#define BG_YELLOW "\033[43m"
#define BG_BLUE "\033[44m"
#define BG_PURPLE "\033[45m"
#define BG_CYAN "\033[46m"
#define BG_WHITE "\033[47m"

#define RESET_COLOR "\033[0m"

// KEYS
#define KEY_LEFT 128
#define KEY_RIGHT 129
#define KEY_UP 130
#define KEY_DOWN 131
#define MOUSE_EVENT 132
#define KEY_FAIL 0

// ENUMS
enum mouse_event {
  IDLE,
  LEFT,
  SPAM_LEFT,
  MIDDLE,
  SPAM_MIDDLE,
  RIGHT,
  SPAM_RIGHT,
  WHEEL_UP,
  WHEEL_DOWN,
};

// STRUCTS
struct mouse {
  int x;
  int y;
  enum mouse_event event;
};

struct cell {
  wchar_t sym;
  char *color;
  int x;
  int y;
};

// TYPEDEF
typedef struct mouse mouse;
typedef struct cell cell;

// EXTERNS
extern int ROWS, COLS;
extern mouse MOUSE;

// FUNCS
void init();
void restore(); // auto atexit
void refresh();
void getsize(int *height, int *width);
void cursset(int visibility);
void setpos(int y, int x);
void posprint(int y, int x, char *s);
void poschar(int y, int x, char c);
void poswchar(int y, int x, wchar_t c);
void clear();
void box(int y, int x, int h, int w, int style);
int getch(int timeout);

// mouse
void enable_mouse();
void disable_mouse();

// WINDOWS

// structs
struct window {
  int id;
  char *name;

  int x;
  int y;
  int width;
  int height;

  int border, filling;

  struct cell **content;
  int content_length, content_offset_x, content_offset_y;

  void (*updater)(struct window *);

  int dragable, drag, drag_ofset;
};

// typedef
typedef struct window window;

// funcs
struct window *new_window(void (*updater)(struct window *));
void wposwchar(struct window *win, int y, int x, wchar_t c);
void wclear(struct window *win);
void render_windows();

#endif // !TERMUTILS_H
