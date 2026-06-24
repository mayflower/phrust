<?php
// stdlib-diff: id=STDLIB_VARIABLE_TYPES area=stdlib expect=pass
echo gettype(null), "|", gettype(false), "|", gettype(7), "|", gettype(1.5), "|", gettype("x"), "|", gettype([1]), "\n";
echo get_debug_type(null), "|", get_debug_type(false), "|", get_debug_type(7), "|", get_debug_type(1.5), "|", get_debug_type("x"), "|", get_debug_type([1]), "\n";
echo is_null(null) ? "1" : "0";
echo is_bool(false) ? "1" : "0";
echo is_int(7) ? "1" : "0";
echo is_float(1.5) ? "1" : "0";
echo is_string("x") ? "1" : "0";
echo is_array([1]) ? "1" : "0";
echo is_scalar("x") ? "1" : "0";
echo is_countable([1]) ? "1" : "0";
echo is_iterable([1]) ? "1" : "0";
echo is_object([1]) ? "1" : "0";
echo "\n";
