# LZJD

Rust implementation of Lempel-Ziv Jaccard Distance (LZJD) algorithm based on [jLZJD](https://github.com/EdwardRaff/jLZJD)

Main differences:
- Rust instead of Java
- Can use any hasher (executable uses CRC32) instead of just Murmur3
- Does not allocate memory for every unique hash, instead keeps k=1024 smallest
- Based on Vec<u64> instead of IntSetNoRemove, which is more like HashMap
- Hash files are considerably smaller if small sequences have been digested

```
USAGE:
    lzjd [FLAGS] [OPTIONS] <INPUT>...

FLAGS:
    -c, --compare        compare SDBFs in file, or two SDBF files
    -r, --deep           generate SDBFs from directories and files
    -g, --gen-compare    compare all pairs in source data
    -h, --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
    -o, --output <FILE>            send output to files
    -t, --threshold <THRESHOLD>    only show results >= threshold [default: 1]

ARGS:
    <INPUT>...    Sets the input file to use
```


See also:

- [Original paper](http://www.edwardraff.com/publications/alternative-ncd-lzjd.pdf)
- [Follow-up paper](https://arxiv.org/abs/1708.03346)