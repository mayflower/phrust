<?php
namespace Phase6\BasicClassmap;

class MappedThing
{
    public function label()
    {
        return \phase6_basic_file_helper('classmap');
    }
}
