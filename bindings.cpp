#include <cwchar> // для wchar_t
#include <pybind11/operators.h>
#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <string>

namespace py = pybind11;

extern "C" {
#include "termutils.h"
}

class PyWindow {
public:
  window_p _win;

  PyWindow(int layer) : _win(new_window(layer)) {}

  ~PyWindow() {
    if (_win) {
      _win = nullptr;
    }
  }

  // Свойства (getters/setters)
  int get_id() const { return _win ? _win->id : -1; }
  std::string get_name() const {
    return _win && _win->name ? std::string(_win->name) : "";
  }
  void set_name(const std::string &name) {
    if (!_win)
      return;
    if (_win->name)
      free(_win->name);
    _win->name = name.empty() ? nullptr : strdup(name.c_str());
  }

  int get_x() const { return _win ? _win->x : 0; }
  void set_x(int v) {
    if (_win)
      _win->x = v;
  }
  int get_y() const { return _win ? _win->y : 0; }
  void set_y(int v) {
    if (_win)
      _win->y = v;
  }
  int get_width() const { return _win ? _win->width : 0; }
  void set_width(int v) {
    if (_win)
      _win->width = v;
  }
  int get_height() const { return _win ? _win->height : 0; }
  void set_height(int v) {
    if (_win)
      _win->height = v;
  }
  int get_always_on_screen() const { return _win ? _win->always_on_sreen : 0; }
  void set_always_on_screen(int v) {
    if (_win)
      _win->always_on_sreen = v;
  }
  int get_fullscreen() const { return _win ? _win->fullscreen : 0; }
  void set_fullscreen(int v) {
    if (_win)
      _win->fullscreen = v;
  }
  int get_layer() const { return _win ? _win->layer : 0; }
  void set_layer(int v) {
    if (_win)
      _win->layer = v;
  }
  int get_border() const { return _win ? _win->border : 0; }
  void set_border(int v) {
    if (_win)
      _win->border = v;
  }
  int get_filling() const { return _win ? _win->filling : 0; }
  void set_filling(int v) {
    if (_win)
      _win->filling = v;
  }
  int get_visible() const { return _win ? _win->visible : 0; }
  void set_visible(int v) {
    if (_win)
      _win->visible = v;
  }
  int get_dragable() const { return _win ? _win->dragable : 0; }
  void set_dragable(int v) {
    if (_win)
      _win->dragable = v;
  }
  int get_clickable() const { return _win ? _win->clickable : 0; }
  void set_clickable(int v) {
    if (_win)
      _win->clickable = v;
  }

  // Runtime свойства (только чтение, обновляются в render)
  bool get_clicked() const { return _win ? _win->clicked : false; }
  int get_clicked_x() const { return _win ? _win->clicked_x : 0; }
  int get_clicked_y() const { return _win ? _win->clicked_y : 0; }

  // Методы
  void pos_wchar(int y, int x, py::str c) {
    if (!_win || c.empty())
      return;
    std::wstring ws = c.cast<std::wstring>();
    if (!ws.empty()) {
      wposwchar(_win, y, x, ws[0]);
    }
  }
  void clear() {
    if (_win)
      wclear(_win);
  }
  void toggle_fullscreen() {
    if (_win)
      wtogglefullscreen(_win);
  }
};

PYBIND11_MODULE(termutils, m) {
  m.doc() = "termutils - Terminal UI library bindings for Python";

  // Константы цветов
  m.attr("FG_BLACK") = py::str(FG_BLACK);
  m.attr("FG_RED") = py::str(FG_RED);
  m.attr("FG_GREEN") = py::str(FG_GREEN);
  m.attr("FG_YELLOW") = py::str(FG_YELLOW);
  m.attr("FG_BLUE") = py::str(FG_BLUE);
  m.attr("FG_PURPLE") = py::str(FG_PURPLE);
  m.attr("FG_CYAN") = py::str(FG_CYAN);
  m.attr("FG_WHITE") = py::str(FG_WHITE);
  m.attr("BG_BLACK") = py::str(BG_BLACK);
  m.attr("BG_RED") = py::str(BG_RED);
  m.attr("BG_GREEN") = py::str(BG_GREEN);
  m.attr("BG_YELLOW") = py::str(BG_YELLOW);
  m.attr("BG_BLUE") = py::str(BG_BLUE);
  m.attr("BG_PURPLE") = py::str(BG_PURPLE);
  m.attr("BG_CYAN") = py::str(BG_CYAN);
  m.attr("BG_WHITE") = py::str(BG_WHITE);
  m.attr("RESET_COLOR") = py::str(RESET_COLOR);

  // Константы клавиш
  m.attr("KEY_LEFT") = py::int_(KEY_LEFT);
  m.attr("KEY_RIGHT") = py::int_(KEY_RIGHT);
  m.attr("KEY_UP") = py::int_(KEY_UP);
  m.attr("KEY_DOWN") = py::int_(KEY_DOWN);
  m.attr("MOUSE_EVENT") = py::int_(MOUSE_EVENT);
  m.attr("KEY_FAIL") = py::int_(KEY_FAIL);

  // Enum MouseEvent
  py::enum_<mouse_event>(m, "MouseEvent")
      .value("IDLE", IDLE)
      .value("LEFT", LEFT)
      .value("SPAM_LEFT", SPAM_LEFT)
      .value("MIDDLE", MIDDLE)
      .value("SPAM_MIDDLE", SPAM_MIDDLE)
      .value("RIGHT", RIGHT)
      .value("SPAM_RIGHT", SPAM_RIGHT)
      .value("WHEEL_UP", WHEEL_UP)
      .value("WHEEL_DOWN", WHEEL_DOWN)
      .export_values();

  // Struct Mouse
  py::class_<mouse>(m, "Mouse")
      .def_readwrite("x", &mouse::x)
      .def_readwrite("y", &mouse::y)
      .def_readwrite("event", &mouse::event)
      .def(py::init<>());

  // Struct Cell (color не используется в коде, но экспонирую)
  py::class_<cell>(m, "Cell")
      .def_readwrite("sym", &cell::sym)
      .def_readwrite("color", &cell::color)
      .def_readwrite("x", &cell::x)
      .def_readwrite("y", &cell::y)
      .def(py::init<>());

  // Глобальные переменные (через функции)
  m.def("get_rows", []() { return ROWS; });
  m.def(
      "set_rows",
      <a href = "int v" target = "_blank" rel = "noopener noreferrer nofollow">
      </ a> { ROWS = v; });
  m.def("get_cols", []() { return COLS; });
  m.def(
      "set_cols",
      <a href = "int v" target = "_blank" rel = "noopener noreferrer nofollow">
      </ a> { COLS = v; });
  m.def("get_mouse",
        []() { return mouse{MOUSE.x, MOUSE.y, MOUSE.event}; }); // Копия

  // Функции
  m.def("init", &init);
  m.def("restore", &restore); // С exit(0)
  m.def("refresh", &refresh);
  m.def("get_size", []() {
    int h = 0, w = 0;
    getsize(&h, &w);
    return py::make_tuple(h, w);
  });
  m.def("cursset", &cursset);
  m.def("setpos", &setpos);
  m.def(
      "posprint", <a href = "int y, int x, py::str s" target = "_blank" rel =
                       "noopener noreferrer nofollow"></ a> {
        if (s.empty())
          return;
        posprint(y, x, const_cast<char *>(s.cast<std::string>().c_str()));
      });
  m.def("poschar", &poschar);
  m.def(
      "poswchar", <a href = "int y, int x, py::str c" target = "_blank" rel =
                       "noopener noreferrer nofollow"></ a> {
        if (c.empty())
          return;
        std::wstring ws = c.cast<std::wstring>();
        if (!ws.empty())
          poswchar(y, x, ws[0]);
      });
  m.def("clear", &clear);
  m.def("box", &box);
  m.def("getch", &getch);
  m.def("enable_mouse", &enable_mouse);
  m.def("disable_mouse", &disable_mouse);
  m.def("render_windows", &render_windows);

  // Класс Window
  py::class_<PyWindow>(m, "Window")
      .def(py::init<int>())
      .def_property_readonly("id", &PyWindow::get_id)
      .def_property("name", &PyWindow::get_name, &PyWindow::set_name)
      .def_property("x", &PyWindow::get_x, &PyWindow::set_x)
      .def_property("y", &PyWindow::get_y, &PyWindow::set_y)
      .def_property("width", &PyWindow::get_width, &PyWindow::set_width)
      .def_property("height", &PyWindow::get_height, &PyWindow::set_height)
      .def_property("always_on_screen", &PyWindow::get_always_on_screen,
                    &PyWindow::set_always_on_screen)
      .def_property("fullscreen", &PyWindow::get_fullscreen,
                    &PyWindow::set_fullscreen)
      .def_property("layer", &PyWindow::get_layer, &PyWindow::set_layer)
      .def_property("border", &PyWindow::get_border, &PyWindow::set_border)
      .def_property("filling", &PyWindow::get_filling, &PyWindow::set_filling)
      .def_property("visible", &PyWindow::get_visible, &PyWindow::set_visible)
      .def_property("dragable", &PyWindow::get_dragable,
                    &PyWindow::set_dragable)
      .def_property("clickable", &PyWindow::get_clickable,
                    &PyWindow::set_clickable)
      .def("get_clicked", &PyWindow::get_clicked)
      .def("get_clicked_x", &PyWindow::get_clicked_x)
      .def("get_clicked_y", &PyWindow::get_clicked_y)
      .def("pos_wchar", &PyWindow::pos_wchar)
      .def("clear", &PyWindow::clear)
      .def("toggle_fullscreen", &PyWindow::toggle_fullscreen);
}
