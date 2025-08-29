#include <stdio.h>
#include <termios.h>
#include <unistd.h>

int main(void) {
  static struct termios original, changed;
  tcgetattr(0, &original);
  changed = original;
  changed.c_lflag &= ~(ICANON | ECHO);
  changed.c_cc[VMIN] = 1;
  changed.c_cc[VTIME] = 0;
  tcsetattr(STDIN_FILENO, TCSANOW, &changed);

  printf("\033[18t");
  fflush(stdout);

  int height, width;
  if (scanf("\033[8;%d;%dt", &height, &width) != 2) {
    fprintf(stderr, "Не удалось получить размер терминала\n");
    width = 80;
    height = 24;
  }
  tcsetattr(STDIN_FILENO, TCSANOW, &original);
  return 0;
}
