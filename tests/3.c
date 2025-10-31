#include <stdio.h>
#include <sys/select.h>
#include <termios.h>
#include <unistd.h>

#define ESC 0x1B

void handle_arrow_keys() {
  char ch1, ch2;

  // Чтение следующих двух символов после ESC
  if (read(STDIN_FILENO, &ch1, 1) == 1 && read(STDIN_FILENO, &ch2, 1) == 1) {
    switch (ch2) {
    case 'A':
      printf("Стрелка вверх нажата\n");
      break;
    case 'B':
      printf("Стрелка вниз нажата\n");
      break;
    case 'C':
      printf("Стрелка вправо нажата\n");
      break;
    case 'D':
      printf("Стрелка влево нажата\n");
      break;
    default:
      // Игнорирование других комбинаций
      break;
    }
  }
}

int main() {
  struct termios oldattr, newattr;
  printf("\033[?1049h");

  // Сохраняем старые атрибуты терминала и устанавливаем новые
  tcgetattr(STDIN_FILENO, &oldattr);
  newattr = oldattr;
  newattr.c_lflag &=
      ~(ICANON | ECHO); // Отключаем канонический режим ввода и эхо
  tcsetattr(STDIN_FILENO, TCSANOW, &newattr);

  printf("Нажмите стрелку для обработки...\n");

  char ch;
  while (1) {
    if (read(STDIN_FILENO, &ch, 1) > 0) {
      if (ch == ESC) { // Начало escape-последовательности
        handle_arrow_keys();
      } else
        printf("%c", ch);
    }
  }

  // Восстанавливаем старые атрибуты терминала
  tcsetattr(STDIN_FILENO, TCSANOW, &oldattr);

  printf("\033[?1049l");
  return 0;
}
