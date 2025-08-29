#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <termios.h>
#include <unistd.h>

void enable_mouse() {
  printf("\033[?1000h\033[?1002h");
  fflush(stdout);
}

void disable_mouse() {
  printf("\033[?1002l\033[?1000l");
  fflush(stdout);
}

// Функция для восстановления настроек терминала
void restore_terminal() {
  struct termios tty;
  tcgetattr(STDIN_FILENO, &tty);
  tty.c_lflag |= (ICANON | ECHO); // Включение канонического режима и эха
  tcsetattr(STDIN_FILENO, TCSANOW, &tty);
}

// Функция для установки терминала в неблокирующий режим
void set_nonblocking() {
  struct termios tty;
  tcgetattr(STDIN_FILENO, &tty);
  tty.c_lflag &= ~(ICANON | ECHO); // Отключение канонического режима и эха
  tcsetattr(STDIN_FILENO, TCSANOW, &tty);

  // Установка stdin в неблокирующий режим
  int flags = fcntl(STDIN_FILENO, F_GETFL, 0);
  fcntl(STDIN_FILENO, F_SETFL, flags | O_NONBLOCK);
}

int main() {
  // Включаем режим мыши и настраиваем терминал
  enable_mouse();
  set_nonblocking();

  printf("Отслеживание событий мыши в консоли. Нажмите Ctrl+C для выхода.\n");

  unsigned char buf[15]; // Буфер для escape-последовательностей
  int bytes;

  while (1) {
    bytes =
        read(STDIN_FILENO, buf, sizeof(buf) - 1); // Оставляем место для '\0'
    if (bytes > 0) {
      buf[bytes] = '\0'; // Завершаем строку

      printf("Необработанные данные: ");
      for (int i = 0; i < bytes; i++) {
        printf("%02x ", (unsigned char)buf[i]);
      }
      printf("\n");
    } else if (bytes == -1 && errno != EAGAIN && errno != EWOULDBLOCK) {
      // Обработка ошибок чтения
      perror("read");
      break;
    }

    usleep(10000); // Небольшая задержка для снижения нагрузки на CPU
  }

  // Отключаем режим мыши и восстанавливаем терминал
  disable_mouse();
  restore_terminal();
  return 0;
}
