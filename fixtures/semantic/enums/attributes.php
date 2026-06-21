<?php

#[Attribute]
class Label
{
}

enum Marked
{
    #[Label]
    case A;
}
