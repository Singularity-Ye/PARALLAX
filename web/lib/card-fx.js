import {motionScope} from './motion.js';
export function initCardFx(selector='.candidate, .launch-card, .paper-stat') {
  const scope=motionScope(),touched=new Set();let frame=0,pointer=null;
  function clear(card){for(const key of ['--spot-x','--spot-y','--tilt-x','--tilt-y'])card.style.removeProperty(key);}
  scope.on(document,'pointermove',event=>{
    if(event.pointerType==='touch')return;
    const card=event.target.closest(selector);if(!card)return;
    pointer={card,x:event.clientX,y:event.clientY};
    if(!frame)frame=scope.frame(()=>{frame=0;const {card,x,y}=pointer;if(!card.isConnected)return;const rect=card.getBoundingClientRect();touched.add(card);
      card.style.setProperty('--spot-x',`${x-rect.left}px`);card.style.setProperty('--spot-y',`${y-rect.top}px`);
      card.style.setProperty('--tilt-x',`${-((y-rect.top)/rect.height-.5)*6.5}deg`);card.style.setProperty('--tilt-y',`${((x-rect.left)/rect.width-.5)*6.5}deg`);
    });
  },{passive:true});
  scope.on(document,'pointerout',event=>{const card=event.target.closest(selector);if(card&&!card.contains(event.relatedTarget)){if(pointer?.card===card){scope.cancel(frame);frame=0;}clear(card);}});
  scope.cleanup(()=>touched.forEach(clear));return ()=>scope.dispose();
}
