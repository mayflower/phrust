<?php
// runtime-semantics: category=types expect=pass
class Counter {
    public static int $value = 1;
}

echo Counter::$value, "\n";
