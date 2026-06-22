<?php
// phase5-runtime: category=errors expect=pass
$e = new TypeError("bad");
echo ($e instanceof Throwable) ? "throwable|" : "no|";
echo ($e instanceof Error) ? "error|" : "no|";
echo ($e instanceof Exception) ? "exception\n" : "not-exception\n";
