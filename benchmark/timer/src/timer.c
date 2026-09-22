#include <mruby.h>

#include <time.h>

static mrb_value
benchmark_timer_monotonic(mrb_state *mrb, mrb_value self)
{
  (void)self;
  struct timespec time;

  if (clock_gettime(CLOCK_MONOTONIC, &time) != 0) {
    mrb_raise(mrb, E_RUNTIME_ERROR, "clock_gettime(CLOCK_MONOTONIC) failed");
  }

  return mrb_float_value(mrb,
    (mrb_float)time.tv_sec + (mrb_float)time.tv_nsec / 1000000000.0);
}

void
mrb_mruby_benchmark_timer_gem_init(mrb_state *mrb)
{
  struct RClass *timer = mrb_define_module(mrb, "BenchmarkTimer");
  mrb_define_module_function(mrb, timer, "monotonic",
                             benchmark_timer_monotonic, MRB_ARGS_NONE());
}

void
mrb_mruby_benchmark_timer_gem_final(mrb_state *mrb)
{
  (void)mrb;
}
