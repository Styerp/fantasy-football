# Who Am I

Fun scripts to help rib your friends!

So we play Fanatsy Football. One day, we wanted to know who put the WORST team out there based on their available roster for the week -- the Bench King!

For a single week, it's pretty easy to eyeball and figure it out, but how many points did you miss out on? What about cumulatively throughout the year? We've got code for that!

Originally written in python, then lost, then written in Typescript/NodeJS, I wanted to do it in Rust, for personal practice. This repository is scripts reliant on my hand written [ESPN Fanatsy Football API Client](https://github.com/Styerp/espn_rust).

Feel free to borrow for your league! I'm open to requests for feature enhancements or other ideas for fun projects. 

## Scripts

### Bench-King

The primary script. This calculates the optimal roster you could have played, based on the players you had on your roster. It calculates the number of points between your optimal and actual and ranks you against your league. If the `-c` flag is passed, all weeks up to the week in question will be calclulated and the comprehensive result will be displayed.