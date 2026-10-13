<?php

namespace App;

class Greeter
{
    public function greet(string $name): string
    {
        // greeting comment
        return "Hello, " . $name;
    }
}

function helper(int $value): int
{
    return $value + 1;
}
