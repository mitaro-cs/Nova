import {expect,it} from 'vitest';
import {levelLabel,qualityName,qualityTier} from './quality-label';
it('labels cropped source resolutions with conventional tiers',()=>{
 expect([136,232,344,456,688,1032].map(v=>qualityTier(v))).toEqual([144,240,360,480,720,1080]);
 expect(qualityTier(800,1920)).toBe(1080);
 expect(qualityTier(1716,3840)).toBe(2160);
});
it('human-readable quality and distinct variants',()=>{
 expect(qualityName(720)).toBe('720p');
 expect(qualityName(2160)).toBe('4K');
 const levels=[{i:0,height:1080},{i:1,height:720},{i:2,height:690}];
 expect(levels.map(l=>levelLabel(l,levels))).toEqual(['1080p','720p','720p (2)']);
 expect(levelLabel({i:0,height:0},levels)).toBe('Оригинал');
});
