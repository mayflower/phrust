<?php
function unpack_values($first, $second, ...$rest) {
    echo $first, "|", $second, "|", $rest[0], "|", $rest[1];
}

unpack_values(...["A", "B"], ...["C", "D"]);
