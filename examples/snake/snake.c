#include <stdlib.h>
#include <termutils.h>

#define SEGMENT_HEIGHT 3
#define SEGMENT_WIDTH 5

typedef struct {
  int snake_length;
  int direction;
  struct window **tail;
} snake;

void segment_updater(struct window *win) {}

void add_segment(snake *s) {
  s->tail = realloc(s->tail, sizeof(struct window *) * (++s->snake_length));
  s->tail[s->snake_length - 1] = new_window(segment_updater);
  s->tail[s->snake_length - 1]->height = SEGMENT_HEIGHT;
  s->tail[s->snake_length - 1]->width = SEGMENT_WIDTH;
  if (s->snake_length - 1) {
    wclear(s->tail[s->snake_length - 2]);
    s->tail[s->snake_length - 2]->border = 0;

    switch (s->direction) {
    case 0:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x + SEGMENT_WIDTH;
      break;
    case 1:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    case 2:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH;
      break;
    case 3:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y + SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    }
  }
  wposwchar(s->tail[s->snake_length - 1], 0, 0, L'\'');
  wposwchar(s->tail[s->snake_length - 1], 0, 2, L'\'');
  wposwchar(s->tail[s->snake_length - 1], 0, 1, L'~');
  s->tail[s->snake_length - 1]->border = 1;
}

void snake_step(snake *s) {
  struct window *last = s->tail[0];
  for (int i = 0; i < s->snake_length - 1; i++) {
    s->tail[i] = s->tail[i + 1];
  }
  wclear(s->tail[s->snake_length - 1]);
  s->tail[s->snake_length - 1]->border = 0;
  s->tail[s->snake_length - 1] = last;
  s->tail[s->snake_length - 1]->border = 1;
  wposwchar(last, 0, 0, L'\'');
  wposwchar(last, 0, 2, L'\'');
  wposwchar(last, 0, 1, L'~');
  if (s->snake_length - 1) {
    switch (s->direction) {
    case 0:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x + SEGMENT_WIDTH;
      break;
    case 1:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    case 2:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH;
      break;
    case 3:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y + SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    }
  }
}

int main() {
  init();
  cursset(0);
  refresh();

  snake *s = malloc(sizeof(snake));
  add_segment(s);
  s->tail[0]->x = 0;
  s->tail[0]->y = 0;
  s->direction = 0;
  add_segment(s);
  add_segment(s);
  add_segment(s);
  add_segment(s);
  render_windows();
  refresh();

  char c;
  while ((c = getch(100)) != 'q') {
    s->direction = c == 'd'   ? 0
                   : c == 'w' ? 1
                   : c == 'a' ? 2
                   : c == 's' ? 3
                              : s->direction;
    if (c == 'e')
      add_segment(s);
    else
      snake_step(s);
    clear();
    render_windows();
    refresh();
  }
  return 0;
}
