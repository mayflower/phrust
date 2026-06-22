<?php
class PropertyState {
    public $x = 0;
    public $y = null;
}
$state = new PropertyState();
echo isset($state->x), isset($state->y), empty($state->x), empty($state->missing);
unset($state->x);
echo isset($state->x), empty($state->x), "\n";
