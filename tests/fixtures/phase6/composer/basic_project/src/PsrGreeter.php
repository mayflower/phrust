<?php
namespace Phase6\Basic;

class PsrGreeter
{
    public function message()
    {
        return \phase6_basic_file_helper('psr4');
    }
}
