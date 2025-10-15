#include <stdio.h>
#include <stdlib.h>
#include <termutils.h>
#include <time.h>
#include <wchar.h>

#define SEGMENT_HEIGHT 3
#define SEGMENT_WIDTH 5

typedef struct {
  int snake_length;
  int direction;

  window **tail;
} snake;

void add_segment(snake *s) {
  s->tail = realloc(s->tail, sizeof(window *) * (++s->snake_length));
  s->tail[s->snake_length - 1] = new_window(1);
  s->tail[s->snake_length - 1]->height = SEGMENT_HEIGHT;
  s->tail[s->snake_length - 1]->width = SEGMENT_WIDTH;
  if (s->snake_length - 1) {
    wclear(s->tail[s->snake_length - 2]);
    s->tail[s->snake_length - 2]->border = 0;
    switch (s->direction) {
    case 0:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x >=
                  COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
              ? 0
              : s->tail[s->snake_length - 2]->x + SEGMENT_WIDTH;
      break;
    case 1:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT < 0
              ? ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
              : s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    case 2:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH < 0
              ? COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
              : s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH;
      break;
    case 3:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y >=
                  ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
              ? 0
              : s->tail[s->snake_length - 2]->y + SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    }
  }
  wposwchar(s->tail[s->snake_length - 1], 0, 0, L'\'');
  wposwchar(s->tail[s->snake_length - 1], 0, 1, L'~');
  wposwchar(s->tail[s->snake_length - 1], 0, 2, L'\'');
  s->tail[s->snake_length - 1]->border = 1;
  s->tail[s->snake_length - 1]->filling = 0;
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
  wposwchar(last, 0, 1, L'~');
  wposwchar(last, 0, 2, L'\'');
  if (s->snake_length - 1) {
    switch (s->direction) {
    case 0:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x >=
                  COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
              ? 0
              : s->tail[s->snake_length - 2]->x + SEGMENT_WIDTH;
      break;
    case 1:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT < 0
              ? ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
              : s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    case 2:
      s->tail[s->snake_length - 1]->y = s->tail[s->snake_length - 2]->y;
      s->tail[s->snake_length - 1]->x =
          s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH < 0
              ? COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
              : s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH;
      break;
    case 3:
      s->tail[s->snake_length - 1]->y =
          s->tail[s->snake_length - 2]->y >=
                  ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
              ? 0
              : s->tail[s->snake_length - 2]->y + SEGMENT_HEIGHT;
      s->tail[s->snake_length - 1]->x = s->tail[s->snake_length - 2]->x;
      break;
    }
  }
}

int collision(window *win1, window *win2) {
  return (win1->y == win2->y && win1->x == win2->x);
}

void change_food_position(window *food, snake *s) {
  int intersection = 1;
  while (intersection) {
    food->y = rand() % (ROWS / SEGMENT_HEIGHT) * SEGMENT_HEIGHT;
    food->x = rand() % (COLS / SEGMENT_WIDTH) * SEGMENT_WIDTH;
    intersection = 0;
    for (int i = 0; i < s->snake_length; i++)
      if (collision(food, s->tail[i])) {
        intersection = 1;
        break;
      }
  }
}

int snake_self_collision(snake *s) {
  for (int i = 0; i < s->snake_length - 1; i++)
    if (collision(s->tail[i], s->tail[s->snake_length - 1]))
      return 1;
  return 0;
}

// int coordinate_collision(window *win, int y, int x) {
//   return (win1->y == win2->y && win1->x == win2->x);
// }

int main() {
  srand(time(NULL));
  int game = 1;
  int pause = 0;
  init();
  cursset(0);
  refresh();

  window *food = new_window(0);
  food->border = -1;
  food->height = SEGMENT_HEIGHT;
  food->width = SEGMENT_WIDTH;
  wposwchar(food, 0, 1, L'󰉛');

  snake *s = malloc(sizeof(snake));
  add_segment(s);
  s->tail[0]->x = 0;
  s->tail[0]->y = 0;
  s->direction = 0;
  int buffer_direction = s->direction;
  for (int i = 0; i < 2; i++)
    add_segment(s);

  change_food_position(food, s);
  render_windows();
  refresh();

  char c;
  int frame;
  while (game) {
    char c1 = getch(2);
    pause = c1 == ' ' ? 1 - pause : pause;
    c = c1 != 0 ? c1 : c;
    s->direction = c == 'd' && buffer_direction != 2   ? 0
                   : c == 'w' && buffer_direction != 3 ? 1
                   : c == 'a' && buffer_direction != 0 ? 2
                   : c == 's' && buffer_direction != 1 ? 3
                                                       : s->direction;

    if (!pause) {
      if (frame % 40 == 0) {
        buffer_direction = s->direction;
        if (collision(s->tail[s->snake_length - 1], food)) {
          change_food_position(food, s);
          add_segment(s);
        } else
          snake_step(s);
        game = snake_self_collision(s) ? 0 : game;
        clear();
        render_windows();
        refresh();
      }

      frame++;
    }
    game = c == 'q' ? 0 : game;
  }
  restore();
  return 0;
}
