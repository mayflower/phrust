<?php
namespace Stdlib\Basic;

class PsrGreeter
{
    public function message()
    {
        return \stdlib_basic_file_helper('psr4');
    }
}
