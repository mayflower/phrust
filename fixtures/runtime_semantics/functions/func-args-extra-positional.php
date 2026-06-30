<?php
function sees_extra($first) {
    echo func_num_args(), "|", func_get_arg(1), "|", implode(",", func_get_args());
}

sees_extra("A", "B", "C");
