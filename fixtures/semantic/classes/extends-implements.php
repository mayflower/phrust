<?php

namespace App\Service;

use Vendor\BaseService;
use Vendor\Contracts\Runnable;
use Vendor\Contracts\Loggable;

class Worker extends BaseService implements Runnable, Loggable
{
}
