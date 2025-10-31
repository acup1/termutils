#include <locale.h>
#include <stdio.h>
#include <wchar.h>

int main() {
  setlocale(LC_ALL, "");
  wchar_t a = L'󱄅';
  printf("%lc", a);
  return 0;
}
