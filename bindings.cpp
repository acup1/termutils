#include <cwchar> // для wchar_t
#include <pybind11/operators.h>
#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <string>

namespace py = pybind11;

extern "C" {
#include "termutils.h"
}

// Класс-обёртка для window
class Window {
public:
  window_p _win;

  Window(int layer) : _win(new_window(layer)) {}

  ~Window() {
    if (_win) {
      del_window(_win);
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
  int get_mouseable() const { return _win ? _win->mouseable : 0; }
  void set_mouseable(int v) {
    if (_win)
      _win->mouseable = v;
  }

  bool mouse() const { return _win ? _win->mouse : false; }
  int mouse_x() const { return _win ? _win->mouse_x : 0; }
  int mouse_y() const { return _win ? _win->mouse_y : 0; }
  enum mouse_event mouse_event() const {
    return _win ? _win->mouse_event : IDLE;
  }

  void pos_wchar(int y, int x, const std::string &c) {
    if (!_win || c.empty())
      return;
    wposwchar(_win, y, x, (wchar_t)strdup(c.c_str())[0]);
  }
  void posprint(int y, int x, const std::string &c) {
    if (!_win || c.empty())
      return;
    wposprint(_win, y, x, strdup(c.c_str()));
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
  m.doc() = "termutils";

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

  m.attr("KEY_LEFT") = py::int_(KEY_LEFT);
  m.attr("KEY_RIGHT") = py::int_(KEY_RIGHT);
  m.attr("KEY_UP") = py::int_(KEY_UP);
  m.attr("KEY_DOWN") = py::int_(KEY_DOWN);
  m.attr("MOUSE_EVENT") = py::int_(MOUSE_EVENT);
  m.attr("KEY_FAIL") = py::int_(KEY_FAIL);

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

  py::class_<mouse>(m, "Mouse")
      .def_readwrite("x", &mouse::x)
      .def_readwrite("y", &mouse::y)
      .def_readwrite("event", &mouse::event)
      .def(py::init<>());

  py::class_<cell>(m, "Cell")
      .def_readwrite("sym", &cell::sym)
      .def_readwrite("color", &cell::color)
      .def_readwrite("x", &cell::x)
      .def_readwrite("y", &cell::y)
      .def(py::init<>());

  m.def("ROWS", []() { return ROWS; }, "height of  terminal");
  m.def("COLS", []() { return COLS; }, "width of terminal");
  // m.attr("ROWS") = ROWS;
  // m.attr("COLS") = COLS;
  m.def("get_mouse", []() { return mouse{MOUSE.x, MOUSE.y, MOUSE.event}; });

  m.def("init", &init);
  m.def("restore", &restore);
  m.def("dell_windows", &del_all_windows);
  m.def("refresh", &refresh);
  m.def("get_size", []() {
    int h = 0, w = 0;
    getsize(&h, &w);
    return py::make_tuple(h, w);
  });
  m.def("cursset", &cursset);
  m.def("setpos", &setpos);
  m.def("posprint", [](int y, int x, py::str s) {
    if (s.cast<std::string>().empty())
      return;
    posprint(y, x, const_cast<char *>(s.cast<std::string>().c_str()));
  });
  m.def("poschar", &poschar);
  m.def("poswchar", [](int y, int x, py::str c) {
    if (c.cast<std::string>().empty())
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

  py::class_<Window>(m, "Window")
      .def(py::init<int>())
      .def_property_readonly("id", &Window::get_id)
      .def_property("name", &Window::get_name, &Window::set_name)
      .def_property("x", &Window::get_x, &Window::set_x)
      .def_property("y", &Window::get_y, &Window::set_y)
      .def_property("width", &Window::get_width, &Window::set_width)
      .def_property("height", &Window::get_height, &Window::set_height)
      .def_property("always_on_screen", &Window::get_always_on_screen,
                    &Window::set_always_on_screen)
      .def_property("fullscreen", &Window::get_fullscreen,
                    &Window::set_fullscreen)
      .def_property("layer", &Window::get_layer, &Window::set_layer)
      .def_property("border", &Window::get_border, &Window::set_border)
      .def_property("filling", &Window::get_filling, &Window::set_filling)
      .def_property("visible", &Window::get_visible, &Window::set_visible)
      .def_property("dragable", &Window::get_dragable, &Window::set_dragable)

      .def_property("mouseable", &Window::get_mouseable, &Window::set_mouseable)
      .def_property_readonly("mouse", &Window::mouse)
      .def_property_readonly("mouse_x", &Window::mouse_x)
      .def_property_readonly("mouse_y", &Window::mouse_y)
      .def_property_readonly("mouse_event", &Window::mouse_event)

      .def("pos_wchar", &Window::pos_wchar)
      .def("posprint", &Window::posprint)
      .def("clear", &Window::clear)
      .def("toggle_fullscreen", &Window::toggle_fullscreen);
}
