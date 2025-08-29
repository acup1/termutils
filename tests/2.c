#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <termios.h>
#include <unistd.h>

static struct termios original, changed;

int main() {
  printf("\033[?1049h");
  tcgetattr(0, &original);
  changed = original;
  changed.c_lflag &= ~(ICANON | ECHO);
  changed.c_cc[VMIN] = 1;
  changed.c_cc[VTIME] = 0;
  tcsetattr(STDIN_FILENO, TCSANOW, &changed);

  // Инициализация структуры poll
  struct pollfd fds[1];
  fds[0].fd = STDIN_FILENO;
  fds[0].events = POLLIN;

  printf("Ожидание ввода до истечения таймаута (5 сек)...\n");

  while (1) {
    int ret = poll(fds, 1, 5); // 5000 миллисекунд
    if (ret == -1) {
      perror("poll");
      exit(EXIT_FAILURE);
    } else if (ret == 0) {
    } else {
      char ch;
      ch = getchar();
      printf("Получен символ: %d\n", ch);
    }
  }

  printf("\033[?1049l");
  tcsetattr(STDIN_FILENO, TCSANOW, &original);
  exit(0);
}
