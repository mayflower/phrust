<?php
function boom()
{
    echo 1 / 0;
}

function wrap()
{
    boom();
}

wrap();
