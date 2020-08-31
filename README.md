# Minesweeper

Simple console based minesweeper. Features a relatively unique feature called
probing.

## Probing

A typical game of minesweeper is not guaranteed to be winnable. Probing makes
it so that any game of minesweeper is winnable.

Upon reaching a state where there is no deterministically correct move left to
make, we allow the player to "probe" the board for a valid move. By doing this,
there is always a path forward for the player, and is therefore always winnable.

However, if the user attempts to probe the board while a valid move still
exists, they will instantly lose the game.

## Searching For Deterministically Correct Moves

In a game of minesweeper, there are a few pieces of information. First, the
board has a set number of mines. Second, there are `n` mines around every
grid cell that is revealed. Third, we mark mines, so, assuming that the player
is correct, there are `n - f` mines around the aforementioned grid cells.

These pieces of information can be represented as the set of cells that are
unknown and the number of mines in that set of cells, namely a CSP.

The problem itself is known as the Minesweeper Consistency Problem, and is NP
Complete, so expect the process to be slow. Shortcircuiting and depth limits
would speed up the process dramatically, but I think the board is typically
small enough that the speed is not of major concern.
