<?php
// runtime-semantics: category=pipe expect=pass
$wrap = fn($value) => "[" . $value . "]";
echo "x" |> $wrap;
