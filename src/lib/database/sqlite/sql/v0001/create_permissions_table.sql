-- This file is part of invman.
--
-- invman - Manage your inventory easily, declaratively, without the headache.
-- Copyright (C) 2023  Maik Steiger <m.steiger@csurielektronics.com>
--
-- invman is free software: you can redistribute it and/or modify
-- it under the terms of the GNU General Public License as published by
-- the Free Software Foundation, either version 3 of the License, or
-- (at your option) any later version.
--
-- invman is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU General Public License for more details.
--
-- You should have received a copy of the GNU General Public License
-- along with invman. If not, see <https://www.gnu.org/licenses/>.
CREATE TABLE invman_permissions(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(8192) NOT NULL UNIQUE
);
