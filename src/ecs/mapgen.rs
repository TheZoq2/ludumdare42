use super::*;
use noise::{NoiseFn, Perlin, Seedable};
use rand;
use std::collections::HashMap;

pub struct GenerateMap;

impl<'a> System<'a> for GenerateMap {
    type SystemData = (
        Entities<'a>,
        Write<'a, Grid>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Tile>,
    );

    fn run(&mut self, (entities, mut grid, mut positions, mut tiles): Self::SystemData) {
        let (w, h, d) = grid.dimensions();
        let noise = Perlin::new().set_seed(rand::random());
        let mut map = HashMap::new();
        for x in 0..w {
            for y in 0..h {
                let bound = (y as f64
                    + 0.5 * d as f64
                        * noise
                            .get([(1.0 + x as f64 / w as f64), (1.0 + y as f64 / h as f64)])
                            .abs())
                    .max(1.0)
                    .min(d as f64 - 1.0);
                for z in 0..(bound as usize) {
                    let entity = entities.create();
                    positions
                        .insert(entity, grid.new_position(entity, x, y, z))
                        .unwrap();
                    tiles.insert(entity, Tile::Terrain).unwrap();
                    map.insert((x, y, z), Tile::Terrain);
                }
            }
        }
        for x in 0..w {
            for y in 0..h {
                for z in 0..4 {
                    if !map.contains_key(&(x, y, z)) {
                        let entity = entities.create();
                        positions
                            .insert(entity, grid.new_position(entity, x, y, z))
                            .unwrap();
                        tiles.insert(entity, Tile::Water).unwrap();
                        map.insert((x, y, z), Tile::Water);
                    }
                }
            }
        }
        for x in 0..w {
            for y in 0..h {
                for z in 4..d {
                    if !map.contains_key(&(x, y, z)) {
                        if map.get(&(x, y, z - 1)) == Some(&Tile::Terrain) {
                            if noise.get([
                                (1.0 + 0.5 * x as f64 / w as f64),
                                (1.0 + 2.0 * y as f64 / h as f64),
                            ]) > 0.0
                            {
                                let entity = entities.create();
                                positions
                                    .insert(entity, grid.new_position(entity, x, y, z))
                                    .unwrap();
                                tiles.insert(entity, Tile::Trees).unwrap();
                                map.insert((x, y, z), Tile::Trees);
                                break;
                            }
                        }
                    }
                }
            }
        }
        grid.current_sealevel = 3;
    }
}

pub struct Flood;

impl<'a> System<'a> for Flood {
    type SystemData = (
        Entities<'a>,
        Write<'a, Grid>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Tile>,
    );

    fn run(&mut self, (entities, mut grid, mut positions, mut tiles): Self::SystemData) {
        let (w, h, d) = grid.dimensions();
        let sealevel = grid.current_sealevel;
        let mut map = HashMap::new();
        {
            let mut floodable = (&*entities, &mut positions, &mut tiles)
                .join()
                .filter(|(_, pos, _)| pos.z() < sealevel + 2)
                .collect::<Vec<_>>();
            let first_plane = floodable
                .iter()
                .filter(|(_, pos, _)| pos.z() == sealevel + 1)
                .collect::<Vec<_>>();
            let first_row = first_plane
                .iter()
                .filter(|(_, pos, _)| pos.y() == 0)
                .collect::<Vec<_>>();
            for x in 0..w {
                if let Some((entity, pos, tile)) = first_row.iter().find(|(_, pos, _)| pos.x() == x)
                {
                    match tile {
                        Tile::Terrain => (),
                        _ => {
                            map.insert((x, 0, sealevel + 1), Some(*entity));
                        }
                    }
                } else {
                    map.insert((x, 0, sealevel + 1), None);
                }
            }
            for y in 1..h {
                for x in 0..w {
                    flood_check_part(
                        x,
                        y,
                        sealevel,
                        (w, h, d),
                        &mut map,
                        &floodable,
                        &first_plane,
                    );
                }
                for x in (0..w).rev() {
                    flood_check_part(
                        x,
                        y,
                        sealevel,
                        (w, h, d),
                        &mut map,
                        &floodable,
                        &first_plane,
                    );
                }
            }
            for y in (1..h).rev() {
                for x in 0..w {
                    flood_check_part(
                        x,
                        y,
                        sealevel,
                        (w, h, d),
                        &mut map,
                        &floodable,
                        &first_plane,
                    );
                }
                for x in (0..w).rev() {
                    flood_check_part(
                        x,
                        y,
                        sealevel,
                        (w, h, d),
                        &mut map,
                        &floodable,
                        &first_plane,
                    );
                }
            }
        }
        for ((x, y, z), entity) in &map {
            if let Some(entity) = entity {
                *tiles.get_mut(*entity).unwrap() = Tile::Water;
            } else {
                let entity = entities.create();
                positions
                    .insert(entity, grid.new_position(entity, *x, *y, *z))
                    .unwrap();
                tiles.insert(entity, Tile::Water).unwrap();
            }
        }
        grid.current_sealevel += 1;
    }
}

fn flood_check_part(
    x: usize,
    y: usize,
    sealevel: usize,
    (w, h, d): (usize, usize, usize),
    map: &mut HashMap<(usize, usize, usize), Option<Entity>>,
    floodable: &Vec<(Entity, &mut Position, &mut Tile)>,
    first_plane: &Vec<&(Entity, &mut Position, &mut Tile)>,
) {
    if !map.contains_key(&(x, y, sealevel + 1))
        && ((y > 0 && map.contains_key(&(x, y - 1, sealevel + 1)))
            || (y < h && map.contains_key(&(x, y + 1, sealevel + 1)))
            || (x > 0 && map.contains_key(&(x - 1, y, sealevel + 1)))
            || (x < w && map.contains_key(&(x + 1, y, sealevel + 1))))
    {
        if let Some((entity, pos, tile)) = first_plane
            .iter()
            .find(|(_, pos, _)| pos.x() == x && pos.y() == y)
        {
            match tile {
                Tile::Terrain => (),
                _ => {
                    map.insert((x, y, sealevel + 1), Some(*entity));
                }
            }
        } else {
            let stack = floodable
                .iter()
                .filter(|(_, pos, _)| pos.x() == x && pos.y() == y)
                .collect::<Vec<_>>();
            for z in 0..(sealevel + 2) {
                if let Some((entity, pos, tile)) = stack.iter().find(|(_, pos, _)| pos.z() == z) {
                    match tile {
                        Tile::Terrain => (),
                        _ => {
                            map.insert((x, y, z), Some(*entity));
                        }
                    }
                } else {
                    map.insert((x, y, z), None);
                }
            }
        }
    }
}
