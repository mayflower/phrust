<?php

function stop(): never
{
    throw new RuntimeException('stop');
}
