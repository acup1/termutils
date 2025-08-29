CC = gcc -Wall -Werror -Wextra

LIB = termutils.so
LIB_SRCS = termutils.c
LIB_OBJS = $(patsubst %.c,objs/%.o,$(LIB_SRCS))

TARGET1 = inittest
SRCS1 = test.c
OBJS1 = $(patsubst %.c,objs/%.o,$(SRCS1))

all: lib test

clean:
	rm -rf $(LIB) $(LIB_OBJS) $(TARGET1) $(OBJS1)

rebuild: clean all

lib: clean_lib $(LIB)

$(LIB): $(LIB_OBJS)
	$(CC) -shared $(LIB_OBJS) -o $@ -lm

objs/%.o: %.c
	@mkdir -p objs
	$(CC) -c $< -o $@

clean_lib: 
	rm -rf $(LIB)

test: $(TARGET1)

$(TARGET1): $(OBJS1) $(LIB)
	$(CC) $< -o $@ -L. $(LIB)

.PHONY: all clean rebuild lib test clean_lib
