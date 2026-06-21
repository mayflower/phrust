<?php

namespace App\Domain;

enum Priority: string implements Labelled
{
    case Low = 'low';
    case High = 'high';
}
