#ifndef FPS_LIMITER_H
#define FPS_LIMITER_H

namespace fallout {
struct FpsLimiter;
} // namespace fallout

extern "C"
{
    fallout::FpsLimiter* rust_create_default_fps_limiter();
    void rust_fps_limiter_mark(fallout::FpsLimiter*);
    void rust_fps_limiter_throttle(fallout::FpsLimiter*);
}

#endif /* FPS_LIMITER_H */
