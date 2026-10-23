import React from 'react';
import {Composition, registerRoot} from 'remotion';
import {FilmComp} from './Film';
import type {Film} from './core';

const empty: Film = {title: 'empty', fps: 30, width: 1920, height: 1080, durationInFrames: 30, chapters: [], shots: [], assets: {}};

/** One composition for every film: the project's build/film.json arrives as input props. */
const Root: React.FC = () => <Composition id="Film" component={FilmComp as unknown as React.FC<Record<string, unknown>>}
  defaultProps={{film: empty}} fps={30} width={1920} height={1080} durationInFrames={30}
  calculateMetadata={({props}) => {
    const film = (props as {film: Film}).film;
    return {durationInFrames: film.durationInFrames, fps: film.fps, width: film.width, height: film.height};
  }} />;
registerRoot(Root);
