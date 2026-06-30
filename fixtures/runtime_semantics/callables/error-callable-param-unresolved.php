<?php
// runtime-semantics: expect=fail
function needs_callable(callable $callback) {
    return $callback();
}

needs_callable("missing_function");
