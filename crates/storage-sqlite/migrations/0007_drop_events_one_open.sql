-- The store enforces one open event among live events (decision #149); the index would count emptied ones.
DROP INDEX events_one_open;
