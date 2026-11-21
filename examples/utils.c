//! We want to display some numbers from the examples to show that they in fact
//! work. However, because of the following issues we can not implement it in
//! Tau directly (yet):
//!
//! - Tau does not have generics yet, so we need a new function for each type
//!   like i64, f64, etc.
//! - Tau does not have c variadics, so we can not directly call printf.
//! - Tau does not have memory buffers or any library for handling memory
//!   manipulations yet, so we can not just write our own formatting function.

#include <stdio.h>
#include <stdint.h>

int32_t print_i64(const char* format, int64_t number) {
    return printf(format, number);
}

int32_t print_f64(const char* format, double number) {
    return printf(format, number);
}

int32_t print_usize(const char* format, size_t number) {
    return printf(format, number);
}
