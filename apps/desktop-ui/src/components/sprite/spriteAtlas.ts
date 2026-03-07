export interface SpriteFrameRect {
  sx: number;
  sy: number;
  sw: number;
  sh: number;
}

export interface SpriteAtlasGrid {
  columns: number;
  rows: number;
  columnBoundaries: number[];
  rowBoundaries: number[];
  frames: SpriteFrameRect[][];
  nominalFrameWidth: number;
  nominalFrameHeight: number;
}

export interface SpriteAtlasValidation {
  warnings: string[];
  errors: string[];
}

export function buildBoundaries(total: number, slices: number): number[] {
  if (!Number.isInteger(total) || total <= 0) {
    throw new Error("total must be a positive integer");
  }

  if (!Number.isInteger(slices) || slices <= 0) {
    throw new Error("slices must be a positive integer");
  }

  const boundaries = new Array<number>(slices + 1);

  for (let index = 0; index <= slices; index += 1) {
    boundaries[index] = Math.round((index * total) / slices);
  }

  boundaries[0] = 0;
  boundaries[slices] = total;

  for (let index = 1; index < boundaries.length; index += 1) {
    if (boundaries[index] < boundaries[index - 1]) {
      boundaries[index] = boundaries[index - 1];
    }
  }

  return boundaries;
}

export function buildAtlasGrid(
  imageWidth: number,
  imageHeight: number,
  columns: number,
  rows: number,
): SpriteAtlasGrid {
  const columnBoundaries = buildBoundaries(imageWidth, columns);
  const rowBoundaries = buildBoundaries(imageHeight, rows);

  const frames: SpriteFrameRect[][] = [];

  for (let row = 0; row < rows; row += 1) {
    const sy = rowBoundaries[row];
    const nextSy = rowBoundaries[row + 1];
    const sh = nextSy - sy;
    const rowFrames: SpriteFrameRect[] = [];

    for (let col = 0; col < columns; col += 1) {
      const sx = columnBoundaries[col];
      const nextSx = columnBoundaries[col + 1];
      const sw = nextSx - sx;

      rowFrames.push({ sx, sy, sw, sh });
    }

    frames.push(rowFrames);
  }

  return {
    columns,
    rows,
    columnBoundaries,
    rowBoundaries,
    frames,
    nominalFrameWidth: Math.round(imageWidth / columns),
    nominalFrameHeight: Math.round(imageHeight / rows),
  };
}

export function getFrameRect(
  grid: SpriteAtlasGrid,
  row: number,
  col: number,
): SpriteFrameRect {
  if (row < 0 || row >= grid.rows || col < 0 || col >= grid.columns) {
    throw new Error(`frame index out of range: row=${row}, col=${col}`);
  }

  return grid.frames[row][col];
}

export function trimFrameTop(
  frame: SpriteFrameRect,
  topPixels: number,
): SpriteFrameRect {
  const safeTopPixels = Math.max(0, Math.min(Math.floor(topPixels), frame.sh - 1));

  return {
    sx: frame.sx,
    sy: frame.sy + safeTopPixels,
    sw: frame.sw,
    sh: frame.sh - safeTopPixels,
  };
}

export function validateAtlas(
  grid: SpriteAtlasGrid,
  imageWidth: number,
  imageHeight: number,
): SpriteAtlasValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  if (imageWidth % grid.columns !== 0) {
    warnings.push(
      `Sprite sheet width ${imageWidth}px is not divisible by ${grid.columns} columns.`,
    );
  }

  if (imageHeight % grid.rows !== 0) {
    warnings.push(
      `Sprite sheet height ${imageHeight}px is not divisible by ${grid.rows} rows.`,
    );
  }

  for (let row = 0; row < grid.rows; row += 1) {
    for (let col = 0; col < grid.columns; col += 1) {
      const frame = grid.frames[row][col];

      if (frame.sw <= 0 || frame.sh <= 0) {
        errors.push(`Frame ${row},${col} has non-positive size.`);
      }

      if (frame.sx < 0 || frame.sy < 0) {
        errors.push(`Frame ${row},${col} starts out of bounds.`);
      }

      if (frame.sx + frame.sw > imageWidth || frame.sy + frame.sh > imageHeight) {
        errors.push(`Frame ${row},${col} exceeds image bounds.`);
      }
    }
  }

  return { warnings, errors };
}
