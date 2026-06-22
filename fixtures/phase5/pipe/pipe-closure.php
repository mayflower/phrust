<?php
// phase5-runtime: category=pipe expect=pass
$wrap = fn($value) => "[" . $value . "]";
echo "x" |> $wrap;
