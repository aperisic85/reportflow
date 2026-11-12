/** Shared hierarchical ordering/numbering for activity lists and saved reports.
 * A section's top-level activities are 1, 2, 3; children are 3.1, 3.2, ...
 * Stable input order is respected. Historical orphan/cyclic links are treated
 * as extra roots instead of hiding records.
 */
export type HierarchyItem={id:string;parent_id:string|null};
export type NumberedItem<T extends HierarchyItem>={item:T;number:string;depth:number};
export function numberedActivities<T extends HierarchyItem>(items:T[]):NumberedItem<T>[]{
 const byId=new Map(items.map(item=>[item.id,item]));
 const children=new Map<string|null,T[]>();
 for(const item of items){
  const parent=item.parent_id&&item.parent_id!==item.id&&byId.has(item.parent_id)?item.parent_id:null;
  const siblings=children.get(parent)??[];
  siblings.push(item);children.set(parent,siblings);
 }
 const result:NumberedItem<T>[]=[];const visited=new Set<string>();let rootIndex=0;
 function walk(item:T,number:string,depth:number){
  if(visited.has(item.id))return;
  visited.add(item.id);result.push({item,number,depth});
  let childIndex=0;
  for(const child of children.get(item.id)??[]){
   if(visited.has(child.id))continue;
   childIndex++;walk(child,`${number}.${childIndex}`,depth+1);
  }
 }
 for(const root of children.get(null)??[]){
  if(visited.has(root.id))continue;
  rootIndex++;walk(root,String(rootIndex),0);
 }
 for(const stray of items){
  if(visited.has(stray.id))continue;
  rootIndex++;walk(stray,String(rootIndex),0);
 }
 return result;
}
