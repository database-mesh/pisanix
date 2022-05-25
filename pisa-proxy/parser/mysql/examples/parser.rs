// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::env;
use mysql_parser::parser::Parser;

fn main() -> io::Result<()>{
    let args: Vec<String> = env::args().collect();
    let lines = read_lines(&args[1])?;
    let p = Parser::new();

    for line in lines {
        if let Ok(line) = line {
            let res = p.parse(&line);
            match res {
                Ok(_data) => {
                    //println!("ok::{:?}", line);
                },
                Err(e) => {
                    println!("err::{:?}::{:?}", line, e);
                }
            }
        }
    }

    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
