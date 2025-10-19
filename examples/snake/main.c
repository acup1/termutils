#include <stdlib.h>
#include <termutils.h>
#include <time.h>

#define SEGMENT_HEIGHT 3
#define SEGMENT_WIDTH 5

typedef struct {
  int snake_length;
  int direction;

  window_p *tail;
} snake;

int game = 1;
int pause = 0;

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
  wposwchar(s->tail[s->snake_length - 1], SEGMENT_HEIGHT / 2 - 1,
            SEGMENT_WIDTH / 2 - 2, L'\'');
  wposwchar(s->tail[s->snake_length - 1], SEGMENT_HEIGHT / 2 - 1,
            SEGMENT_WIDTH / 2 - 1, L'~');
  wposwchar(s->tail[s->snake_length - 1], SEGMENT_HEIGHT / 2 - 1,
            SEGMENT_WIDTH / 2, L'\'');
  s->tail[s->snake_length - 1]->border = 1;
  s->tail[s->snake_length - 1]->filling = 0;
}

void snake_step(snake *s) {
  window_p last = s->tail[0];
  for (int i = 0; i < s->snake_length - 1; i++) {
    s->tail[i] = s->tail[i + 1];
  }
  wclear(s->tail[s->snake_length - 1]);
  s->tail[s->snake_length - 1]->border = 0;
  s->tail[s->snake_length - 1] = last;
  last->border = 1;
  wposwchar(last, SEGMENT_HEIGHT / 2 - 1, SEGMENT_WIDTH / 2 - 2, L'\'');
  wposwchar(last, SEGMENT_HEIGHT / 2 - 1, SEGMENT_WIDTH / 2 - 1, L'~');
  wposwchar(last, SEGMENT_HEIGHT / 2 - 1, SEGMENT_WIDTH / 2, L'\'');

  if (s->snake_length - 1) {
    switch (s->direction) {
    case 0:
      last->y = s->tail[s->snake_length - 2]->y;
      last->x = s->tail[s->snake_length - 2]->x >=
                        COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
                    ? 0
                    : s->tail[s->snake_length - 2]->x + SEGMENT_WIDTH;
      break;
    case 1:
      last->y = s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT < 0
                    ? ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
                    : s->tail[s->snake_length - 2]->y - SEGMENT_HEIGHT;
      last->x = s->tail[s->snake_length - 2]->x;
      break;
    case 2:
      last->y = s->tail[s->snake_length - 2]->y;
      last->x = s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH < 0
                    ? COLS / SEGMENT_WIDTH * SEGMENT_WIDTH - SEGMENT_WIDTH
                    : s->tail[s->snake_length - 2]->x - SEGMENT_WIDTH;
      break;
    case 3:
      last->y = s->tail[s->snake_length - 2]->y >=
                        ROWS / SEGMENT_HEIGHT * SEGMENT_HEIGHT - SEGMENT_HEIGHT
                    ? 0
                    : s->tail[s->snake_length - 2]->y + SEGMENT_HEIGHT;
      last->x = s->tail[s->snake_length - 2]->x;
      break;
    default:
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

void menu_bg_updater(window_p win) {
  wclear(win);
  for (int i = 0; i < win->height - 3; i++)
    for (int j = 0; j < win->width - 3; j++)
      if ((i + j) % SEGMENT_WIDTH == 0)
        wposwchar(win, i, j, L'*');
}
void menu_continue_btn_updater(window_p win) {
  win->x = COLS / 2 - 5;
  win->y = ROWS / 2 - 1;
  if (win->clicked)
    pause = 0;
}

int main() {
  srand(time(NULL));
  init();
  cursset(0);
  enable_mouse();
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

  window_p menu_bg = new_window(100);
  menu_bg->updater = menu_bg_updater;
  menu_bg->x = 0;
  menu_bg->y = 0;
  menu_bg->width = 0;
  menu_bg->height = 0;
  menu_bg->filling = 0;
  menu_bg->visible = 0;
  wtogglefullscreen(menu_bg);
  window_p menu_continue_btn = new_window(101);
  menu_continue_btn->name = "continue";
  menu_continue_btn->width = 10;
  menu_continue_btn->height = 2;
  menu_continue_btn->visible = 0;
  menu_continue_btn->clickable = 1;
  menu_continue_btn->updater = menu_continue_btn_updater;

  render_windows();
  refresh();

  char c;
  int frame;
  while (game) {
    char c1 = getch(2, 1);
    pause = c1 == ' ' ? 1 - pause : pause;
    if (pause) {
      menu_bg->visible = 1;
      menu_continue_btn->visible = 1;
    } else {
      menu_bg->visible = 0;
      menu_continue_btn->visible = 0;
    }
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
        frame = 0;
      }
    } else {
      if (frame % 20 == 0) {
        clear();
        render_windows();
        refresh();
        frame = 0;
      }
    }
    frame++;
    game = c == 'q' ? 0 : game;
  }
  restore();
  return 0;
}
