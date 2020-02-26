
### Overview 
This project includes several parts:
 * A [Wikipidia XML dump](https://en.wikipedia.org/wiki/Wikipedia:Database_download) parser implemented in rust that 
   produces an intermediate link graph file.
 * An analysis tool implemented in rust that reads in the intermediate link graph file and produces several outputs
   (see analyze.rs for docs with more information). 
 * A python jupyter notebook to process the output of the rust analysis tool
 
### Running
 The rust parser and analysis tool are accessible through a CLI. 
 Make sure you compile this project as release (optimized) or otherwise it will be apocalyptically slow.
 Expect parsing to take 30 minutes to 1 hour depending on your disk IO performance. 
 16GB of ram is recommended for parsing to avoid swapping too much.
 
 To access the CLI help run `wikipedia-analysis --help` or `wikipedia-analysis <subcommand> --help`. 
 Additional help/explanation is available as rustdoc in the code and may be compiled to html using cargo.
 
### Motivation
The purpose of this project was:
* I was inspired by the [Wiki Game](https://en.wikipedia.org/wiki/Wikipedia:Wiki_Game) and wanted to find out how 
  connected the Wikipedia link graph was.
* I thought this project was a good application for rust. 
  I have previously implemented this project in python and C++, so wanted to try it in rust to help learn the language.

### Results 
I will create a more thorough writeup with results and link it here when its ready.