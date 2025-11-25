按照以下要求优化"./fetch_luogu_problem.py"和"./judge.py"

修改几个问题
1. 获取的tagname不应该是序号而是名称比如USACO,NOIP普及组等等.
2. fetch里生成html的代码和html文件的代码有重复,需要减少重复代码,检查代码保证简洁无错误
3. html界面要设计一个完善的夜间模式,并提供切换按键
4. 重整项目结构为如下
   / - script
      | - fetch_luogu_problem.py
      | - judge.py
      | - luogu_catalog.html
      | - luogu_problems.json
     - problem
      | - P...
         | - main.cpp
         | - sample1.in
         | - sample1.out
         | - T.md
     - README.md(为这个项目写一个readme介绍)